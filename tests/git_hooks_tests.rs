// only run this test on Linux
#![cfg(target_os = "linux")]
use anyhow::Result;
use git2::Repository;
use git_iris::git::GitRepo;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

fn setup_git_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create TempDir");
    let repo = Repository::init(temp_dir.path()).expect("Failed to initialize repository");

    // Configure git user
    let mut config = repo.config().expect("Failed to get repository config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user.name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user.email");

    // Create and commit an initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Initial content").expect("Failed to write initial content");

    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("initial.txt"))
        .expect("Failed to add path to index");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let signature = repo.signature().expect("Failed to get signature");
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .expect("Failed to commit");

    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    (temp_dir, git_repo)
}

// Helper function to create a hook script
fn create_hook(repo_path: &Path, hook_name: &str, content: &str, should_fail: bool) -> Result<()> {
    let hooks_dir = repo_path.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join(hook_name);
    let mut file = File::create(&hook_path)?;
    writeln!(file, "#!/bin/sh")?;
    writeln!(file, "echo \"Running {hook_name} hook\"")?;
    writeln!(file, "{content}")?;
    if should_fail {
        writeln!(file, "exit 1")?;
    } else {
        writeln!(file, "exit 0")?;
    }
    file.flush()?;

    // Make the hook executable
    let mut perms = fs::metadata(&hook_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms)?;

    Ok(())
}

#[test]
fn test_verify_and_commit_success() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create successful pre-commit and post-commit hooks
    create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks passed\"",
        false,
    )?;
    create_hook(
        repo_path,
        "post-commit",
        "echo \"Post-commit tasks completed\"",
        false,
    )?;

    // Create and stage a new file
    let new_file_path = repo_path.join("test_file.txt");
    fs::write(&new_file_path, "Test content").expect("Failed to write test content");
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("test_file.txt"))
        .expect("Failed to add path to index");
    index.write().expect("Failed to write index");

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(precommit.is_ok(), "Pre-commit hook should succeed");

    // Perform commit_and_verify
    let result = git_repo.commit_and_verify("Test commit message");

    assert!(result.is_ok(), "verify_and_commit should succeed");
    let commit_result = result.expect("Commit failed");
    assert_eq!(commit_result.files_changed, 1);
    assert!(!commit_result.commit_hash.is_empty());

    Ok(())
}

#[test]
fn test_verify_and_commit_pre_commit_failure() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create a failing pre-commit hook
    create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks failed\"",
        true,
    )?;

    // Create and stage a new file
    let new_file_path = repo_path.join("test_file.txt");
    fs::write(&new_file_path, "Test content").expect("Failed to write test content");
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("test_file.txt"))
        .expect("Failed to add path to index");
    index.write().expect("Failed to write index");

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(
        precommit.is_err(),
        "Commit should fail due to pre-commit hook"
    );

    // Verify that no commit was made
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Initial commit"
    );

    Ok(())
}

#[test]
fn test_verify_and_commit_post_commit_failure() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create successful pre-commit and failing post-commit hooks
    create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks passed\"",
        false,
    )?;
    create_hook(
        repo_path,
        "post-commit",
        "echo \"Post-commit tasks failed\"",
        true,
    )?;

    // Create and stage a new file
    let new_file_path = repo_path.join("test_file.txt");
    fs::write(&new_file_path, "Test content").expect("Failed to write test content");
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("test_file.txt"))
        .expect("Failed to add path to index");
    index.write().expect("Failed to write index");

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(precommit.is_ok(), "Pre-commit hook should succeed");

    // Perform commit_and_verify
    let result = git_repo.commit_and_verify("Test commit message");

    // The commit should succeed even if the post-commit hook fails
    assert!(
        result.is_ok(),
        "verify_and_commit should succeed despite post-commit hook failure"
    );
    let commit_result = result.expect("Commit failed");
    assert_eq!(commit_result.files_changed, 1);
    assert!(!commit_result.commit_hash.is_empty());

    // Verify that the commit was made
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Test commit message"
    );

    Ok(())
}

#[test]
fn test_verify_and_commit_no_hooks() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create and stage a new file
    let new_file_path = repo_path.join("test_file.txt");
    fs::write(&new_file_path, "Test content").expect("Failed to write test content");
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("test_file.txt"))
        .expect("Failed to add path to index");
    index.write().expect("Failed to write index");

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(precommit.is_ok(), "Pre-commit hook should succeed");

    // Perform commit_and_verify
    let result = git_repo.commit_and_verify("Test commit message");

    assert!(
        result.is_ok(),
        "verify_and_commit should succeed without hooks"
    );
    let commit_result = result.expect("Commit failed");
    assert_eq!(commit_result.files_changed, 1);
    assert!(!commit_result.commit_hash.is_empty());

    // Verify that the commit was made
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Test commit message"
    );

    Ok(())
}
