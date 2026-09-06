use git_iris::agents::tools::common::{current_repo_execution_trusted, current_repo_root};
use git_iris::agents::tools::docs::{DocType, ProjectDocs, ProjectDocsArgs};
use git_iris::agents::tools::static_analysis::{
    StaticAnalysis, StaticAnalysisArgs, StaticAnalyzer,
};
use git_iris::agents::tools::{with_active_repo_root, with_repo_execution_context};
use rig::tool::portable::PortableTool;
use tempfile::TempDir;

fn analysis_args() -> StaticAnalysisArgs {
    StaticAnalysisArgs {
        analyzer: StaticAnalyzer::Auto,
        timeout_secs: 1,
        max_output_chars: 512,
    }
}

#[tokio::test]
async fn remote_execution_context_blocks_project_code() -> anyhow::Result<()> {
    let repo = TempDir::new()?;
    std::fs::write(repo.path().join("Cargo.toml"), "[package]\nname='remote'\n")?;
    let result = with_repo_execution_context(repo.path(), false, async {
        StaticAnalysis.call(analysis_args()).await
    })
    .await;
    assert!(
        result
            .expect_err("Remote code must not execute")
            .to_string()
            .contains("untrusted")
    );
    Ok(())
}

#[tokio::test]
async fn local_execution_context_retains_static_analysis() -> anyhow::Result<()> {
    let repo = TempDir::new()?;
    let result = with_repo_execution_context(repo.path(), true, async {
        StaticAnalysis.call(analysis_args()).await
    })
    .await?;
    assert!(result.contains("No matching project markers"));
    Ok(())
}

#[tokio::test]
async fn concurrent_repository_contexts_keep_trust_isolated() -> anyhow::Result<()> {
    let remote = TempDir::new()?;
    let local = TempDir::new()?;
    let (remote_result, local_result) = tokio::join!(
        with_repo_execution_context(remote.path(), false, async {
            with_active_repo_root(remote.path(), async {
                tokio::task::yield_now().await;
                assert_eq!(current_repo_root()?, remote.path());
                assert!(!current_repo_execution_trusted());
                Ok::<_, anyhow::Error>(())
            })
            .await
        }),
        with_repo_execution_context(local.path(), true, async {
            tokio::task::yield_now().await;
            assert_eq!(current_repo_root()?, local.path());
            assert!(current_repo_execution_trusted());
            Ok::<_, anyhow::Error>(())
        }),
    );
    remote_result?;
    local_result?;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn project_docs_do_not_follow_external_symlinks() -> anyhow::Result<()> {
    let repo = TempDir::new()?;
    let external = TempDir::new()?;
    let secret = "EXTERNAL_DOCUMENT_SENTINEL";
    std::fs::write(external.path().join("private.txt"), secret)?;
    for filename in ["README.md", "AGENTS.md"] {
        std::os::unix::fs::symlink(
            external.path().join("private.txt"),
            repo.path().join(filename),
        )?;
    }
    for doc_type in [
        DocType::Readme,
        DocType::Agents,
        DocType::Context,
        DocType::All,
    ] {
        let result = with_active_repo_root(repo.path(), async {
            ProjectDocs
                .call(ProjectDocsArgs {
                    doc_type,
                    max_chars: 20_000,
                })
                .await
        })
        .await;
        let output = match result {
            Ok(output) => output,
            Err(error) => error.to_string(),
        };
        assert!(!output.contains(secret));
    }
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn project_docs_follow_internal_instruction_symlinks() -> anyhow::Result<()> {
    let repo = TempDir::new()?;
    std::fs::write(
        repo.path().join("CLAUDE.md"),
        "# Instructions\nTrusted local instructions.",
    )?;
    std::os::unix::fs::symlink("CLAUDE.md", repo.path().join("AGENTS.md"))?;
    for doc_type in [DocType::Agents, DocType::Context, DocType::All] {
        let output = with_active_repo_root(repo.path(), async {
            ProjectDocs
                .call(ProjectDocsArgs {
                    doc_type,
                    max_chars: 20_000,
                })
                .await
        })
        .await?;
        assert!(output.contains("Trusted local instructions."));
    }
    Ok(())
}
