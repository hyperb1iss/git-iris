// only run this test on Linux
#![cfg(target_os = "linux")]
use anyhow::Result;
use git2::Repository;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitHooksTestHelper, GitTestHelper, setup_git_repo};

#[test]
fn test_verify_and_commit_success() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create successful pre-commit and post-commit hooks using our helper
    GitHooksTestHelper::create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks passed\"",
        false,
    )?;
    GitHooksTestHelper::create_hook(
        repo_path,
        "post-commit",
        "echo \"Post-commit tasks completed\"",
        false,
    )?;

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

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

    // Create a failing pre-commit hook using our helper
    GitHooksTestHelper::create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks failed\"",
        true,
    )?;

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

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

    // Create successful pre-commit and failing post-commit hooks using our helper
    GitHooksTestHelper::create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks passed\"",
        false,
    )?;
    GitHooksTestHelper::create_hook(
        repo_path,
        "post-commit",
        "echo \"Post-commit tasks failed\"",
        true,
    )?;

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

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

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

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

fn write_executable_hook(path: &std::path::Path, body: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().expect("Hook path has a parent"))?;
    std::fs::write(path, format!("#!/bin/sh\n{body}\n"))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[test]
fn configured_relative_hooks_path_is_respected() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo = Repository::open(temp_dir.path())?;
    repo.config()?.set_str("core.hooksPath", ".custom hooks")?;
    write_executable_hook(&temp_dir.path().join(".custom hooks/pre-commit"), "exit 23")?;
    assert!(git_repo.execute_hook("pre-commit").is_err());
    Ok(())
}

#[test]
fn configured_absolute_hooks_path_is_respected() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let hooks_dir = tempfile::TempDir::new()?;
    let repo = Repository::open(temp_dir.path())?;
    repo.config()?.set_str(
        "core.hooksPath",
        hooks_dir.path().to_str().expect("UTF-8 temp path"),
    )?;
    write_executable_hook(&hooks_dir.path().join("pre-commit"), "exit 23")?;
    assert!(git_repo.execute_hook("pre-commit").is_err());
    Ok(())
}

#[test]
fn linked_worktree_uses_shared_hooks_with_worktree_environment() -> Result<()> {
    let (temp_dir, _) = setup_git_repo();
    let repo = Repository::open(temp_dir.path())?;
    let checkout_parent = tempfile::TempDir::new()?;
    let checkout = checkout_parent.path().join("checkout");
    repo.worktree("hook-test", &checkout, None)?;
    write_executable_hook(
        &repo.path().join("hooks/pre-commit"),
        "test \"$(git rev-parse --show-toplevel)\" = \"$PWD\" || exit 24\nprintf 'hook ran' > hook-marker",
    )?;
    let linked_repo = git_iris::git::GitRepo::new(&checkout)?;
    linked_repo.execute_hook("pre-commit")?;
    assert_eq!(
        std::fs::read_to_string(checkout.join("hook-marker"))?,
        "hook ran"
    );
    assert!(!temp_dir.path().join("hook-marker").exists());
    Ok(())
}

#[test]
fn non_executable_hooks_are_ignored() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let (temp_dir, git_repo) = setup_git_repo();
    let hook = temp_dir.path().join(".git/hooks/pre-commit");
    std::fs::write(&hook, "#!/bin/sh\nexit 23\n")?;
    std::fs::set_permissions(hook, std::fs::Permissions::from_mode(0o644))?;
    git_repo.execute_hook("pre-commit")?;
    Ok(())
}
