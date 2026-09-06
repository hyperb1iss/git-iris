use std::fs;
use std::process::Command;

use rig::tool::portable::PortableTool;
use tempfile::TempDir;

use crate::agents::tools::git::{GitShow, GitShowArgs};
use crate::agents::tools::with_active_repo_root;

fn run_git(temp_dir: &TempDir, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(temp_dir.path())
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repo_with_commits() -> TempDir {
    let temp_dir = TempDir::new().expect("temp dir should be created");
    run_git(&temp_dir, &["init"]);
    run_git(&temp_dir, &["config", "user.name", "Iris Tester"]);
    run_git(&temp_dir, &["config", "user.email", "iris@example.com"]);
    run_git(&temp_dir, &["config", "commit.gpgsign", "false"]);
    run_git(&temp_dir, &["config", "tag.gpgsign", "false"]);

    fs::create_dir_all(temp_dir.path().join("src")).expect("src dir should be created");
    fs::write(temp_dir.path().join("src/lib.rs"), "pub fn first() {}\n")
        .expect("source file should be written");
    fs::write(temp_dir.path().join("src/old.rs"), "pub fn old() {}\n")
        .expect("old source file should be written");
    fs::write(temp_dir.path().join("README.md"), "# Demo\n").expect("README should be written");
    run_git(&temp_dir, &["add", "."]);
    run_git(&temp_dir, &["commit", "-m", "Add initial files"]);

    fs::write(
        temp_dir.path().join("src/lib.rs"),
        "pub fn first() {}\npub fn second() {}\n",
    )
    .expect("source file should be updated");
    fs::write(temp_dir.path().join("README.md"), "# Demo\n\nUpdated\n")
        .expect("README should be updated");
    fs::remove_file(temp_dir.path().join("src/old.rs")).expect("old source file should be removed");
    run_git(&temp_dir, &["add", "."]);
    run_git(&temp_dir, &["commit", "-m", "Extend demo files"]);

    temp_dir
}

#[tokio::test]
async fn git_show_returns_commit_patch_and_metadata() {
    let temp_dir = repo_with_commits();

    let output = with_active_repo_root(temp_dir.path(), async {
        GitShow
            .call(GitShowArgs {
                commit: "HEAD".to_string(),
                files: None,
                max_output_chars: 20_000,
            })
            .await
            .expect("git show should succeed")
    })
    .await;

    assert!(output.contains("Git show for HEAD"));
    assert!(output.contains("Author:"));
    assert!(output.contains("Commit:"));
    assert!(output.contains("Extend demo files"));
    assert!(output.contains("+pub fn second() {}"));
}

#[tokio::test]
async fn git_show_filters_paths() {
    let temp_dir = repo_with_commits();

    let output = with_active_repo_root(temp_dir.path(), async {
        GitShow
            .call(GitShowArgs {
                commit: "HEAD".to_string(),
                files: Some(vec!["src/lib.rs".into()]),
                max_output_chars: 20_000,
            })
            .await
            .expect("git show should succeed")
    })
    .await;

    assert!(output.contains("Filtered paths: src/lib.rs"));
    assert!(output.contains("+pub fn second() {}"));
    assert!(!output.contains("README.md"));
}

#[tokio::test]
async fn git_show_filters_historical_paths_missing_from_worktree() {
    let temp_dir = repo_with_commits();

    let output = with_active_repo_root(temp_dir.path(), async {
        GitShow
            .call(GitShowArgs {
                commit: "HEAD^".to_string(),
                files: Some(vec!["src/old.rs".into()]),
                max_output_chars: 20_000,
            })
            .await
            .expect("historical paths should not need to exist in the worktree")
    })
    .await;

    assert!(output.contains("Filtered paths: src/old.rs"));
    assert!(output.contains("+pub fn old() {}"));
}

#[tokio::test]
async fn git_show_rejects_option_like_commits() {
    let temp_dir = repo_with_commits();

    let error = with_active_repo_root(temp_dir.path(), async {
        GitShow
            .call(GitShowArgs {
                commit: "--help".to_string(),
                files: None,
                max_output_chars: 20_000,
            })
            .await
            .expect_err("option-like refs should be rejected")
    })
    .await;

    assert!(error.to_string().contains("commit, tag, or branch"));
}

#[tokio::test]
async fn git_show_rejects_parent_directory_paths() {
    let temp_dir = repo_with_commits();

    let error = with_active_repo_root(temp_dir.path(), async {
        GitShow
            .call(GitShowArgs {
                commit: "HEAD".to_string(),
                files: Some(vec!["../README.md".into()]),
                max_output_chars: 20_000,
            })
            .await
            .expect_err("parent paths should be rejected")
    })
    .await;

    assert!(error.to_string().contains("repository-relative"));
}
