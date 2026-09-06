use std::fs;
use std::process::Command;

use rig::tool::portable::PortableTool;
use tempfile::TempDir;

use crate::agents::tools::git::{GitBlame, GitBlameArgs};
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

fn repo_with_history() -> TempDir {
    let temp_dir = TempDir::new().expect("temp dir should be created");
    run_git(&temp_dir, &["init"]);
    run_git(&temp_dir, &["config", "user.name", "Iris Tester"]);
    run_git(&temp_dir, &["config", "user.email", "iris@example.com"]);
    run_git(&temp_dir, &["config", "commit.gpgsign", "false"]);
    run_git(&temp_dir, &["config", "tag.gpgsign", "false"]);

    fs::create_dir_all(temp_dir.path().join("src")).expect("src dir should be created");
    fs::write(
        temp_dir.path().join("src/lib.rs"),
        "pub fn first() {}\npub fn second() {}\n",
    )
    .expect("source file should be written");
    run_git(&temp_dir, &["add", "src/lib.rs"]);
    run_git(&temp_dir, &["commit", "-m", "Add library functions"]);

    fs::write(
        temp_dir.path().join("src/lib.rs"),
        "pub fn first() {}\npub fn second() {}\npub fn third() {}\n",
    )
    .expect("source file should be updated");
    run_git(&temp_dir, &["add", "src/lib.rs"]);
    run_git(&temp_dir, &["commit", "-m", "Extend library functions"]);

    temp_dir
}

#[tokio::test]
async fn git_blame_returns_line_history_and_recent_file_commits() {
    let temp_dir = repo_with_history();

    let output = with_active_repo_root(temp_dir.path(), async {
        GitBlame
            .call(GitBlameArgs {
                file: "src/lib.rs".into(),
                start_line: 2,
                end_line: Some(3),
                recent_commits: 2,
            })
            .await
            .expect("git blame should succeed")
    })
    .await;

    assert!(output.contains("Git blame for src/lib.rs:2-3"));
    assert!(output.contains("Code:"));
    assert!(output.contains("2 | pub fn second() {}"));
    assert!(output.contains("Blame commits:"));
    assert!(output.contains("Add library functions"));
    assert!(output.contains("Recent commits touching this file:"));
    assert!(output.contains("Extend library functions"));
}

#[tokio::test]
async fn git_blame_rejects_parent_directory_paths() {
    let temp_dir = repo_with_history();

    let error = with_active_repo_root(temp_dir.path(), async {
        GitBlame
            .call(GitBlameArgs {
                file: "../outside.rs".into(),
                start_line: 1,
                end_line: None,
                recent_commits: 3,
            })
            .await
            .expect_err("parent paths should be rejected")
    })
    .await;

    assert!(error.to_string().contains("repository-relative"));
}

#[tokio::test]
async fn git_blame_rejects_absolute_paths() {
    let temp_dir = repo_with_history();

    let error = with_active_repo_root(temp_dir.path(), async {
        GitBlame
            .call(GitBlameArgs {
                file: temp_dir.path().join("src/lib.rs"),
                start_line: 1,
                end_line: None,
                recent_commits: 3,
            })
            .await
            .expect_err("absolute paths should be rejected")
    })
    .await;

    assert!(error.to_string().contains("repository-relative"));
}

#[tokio::test]
async fn git_blame_softens_out_of_range_lines() {
    let temp_dir = repo_with_history();

    let output = with_active_repo_root(temp_dir.path(), async {
        GitBlame
            .call(GitBlameArgs {
                file: "src/lib.rs".into(),
                start_line: 100,
                end_line: Some(101),
                recent_commits: 1,
            })
            .await
            .expect("out-of-range lines should still return file history")
    })
    .await;

    assert!(output.contains("<line range outside file: src/lib.rs>"));
    assert!(output.contains("- No blame data found"));
    assert!(output.contains("Recent commits touching this file:"));
    assert!(output.contains("Extend library functions"));
}
