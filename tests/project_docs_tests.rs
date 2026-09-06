#![allow(clippy::unwrap_used)]

use git_iris::agents::tools::docs::{DocType, ProjectDocsArgs};
use git_iris::agents::tools::{ProjectDocs, with_active_repo_root};
use rig::tool::portable::PortableTool;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn context_returns_a_compact_prioritized_snapshot() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");

    fs::write(
        temp_dir.path().join("README.md"),
        r#"# Git-Iris

AI-powered Git workflows for teams shipping real code.

## Usage

Use `git-iris commit`, `git-iris review`, and `git-iris pr` to generate focused output from Git context.

## Testing

Run `cargo test` and `cargo clippy --all-targets --all-features -- -W clippy::pedantic` before shipping.

## Appendix

DO_NOT_INCLUDE_README_APPENDIX

```bash
echo "DO_NOT_INCLUDE_CODE_BLOCK"
```
"#,
    )
    .expect("Failed to write README");

    fs::write(
        temp_dir.path().join("AGENTS.md"),
        r"# Git-Iris Guide

Top-level style notes that should not dominate context gathering.

## Project Ecosystem

Git-Iris is a Rust TUI and agent system centered on Git workflows.

## Testing Conventions

Run `cargo fmt --all`, `cargo test`, and `cargo clippy --all-targets --all-features -- -W clippy::pedantic`.

## Multi-Agent Git Hygiene

Commit only the files you changed and never push from an agent session.

## Appendix

DO_NOT_INCLUDE_AGENTS_APPENDIX
",
    )
    .expect("Failed to write AGENTS.md");

    let docs = with_active_repo_root(temp_dir.path(), async {
        ProjectDocs
            .call(ProjectDocsArgs {
                doc_type: DocType::Context,
                max_chars: 20_000,
            })
            .await
            .expect("Failed to read compact context")
    })
    .await;

    assert!(docs.starts_with("Concise project context."));
    assert!(docs.contains("=== README.md ==="));
    assert!(docs.contains("=== AGENTS.md ==="));
    assert!(docs.contains("Key sections:"));
    assert!(docs.contains("Highlights:"));
    assert!(docs.contains("Usage"));
    assert!(docs.contains("Testing Conventions"));
    assert!(docs.contains("project_docs(doc_type=\"readme\")"));
    assert!(!docs.contains("DO_NOT_INCLUDE_README_APPENDIX"));
    assert!(!docs.contains("DO_NOT_INCLUDE_AGENTS_APPENDIX"));
    assert!(!docs.contains("DO_NOT_INCLUDE_CODE_BLOCK"));
    assert!(docs.chars().count() <= 8_003);
}

#[tokio::test]
async fn context_respects_a_smaller_total_budget() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");

    fs::write(
        temp_dir.path().join("README.md"),
        "# Title\n\nA very long summary paragraph that keeps going so we can verify truncation behavior.\n",
    )
    .expect("Failed to write README");

    let docs = with_active_repo_root(temp_dir.path(), async {
        ProjectDocs
            .call(ProjectDocsArgs {
                doc_type: DocType::Context,
                max_chars: 120,
            })
            .await
            .expect("Failed to read compact context")
    })
    .await;

    assert!(docs.chars().count() <= 123);
    assert!(docs.starts_with("Concise project context."));
}
