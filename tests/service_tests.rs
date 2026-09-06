use anyhow::Result;
use git_iris::git::GitRepo;
use git_iris::services::GitCommitService;
use std::sync::Arc;
use tempfile::TempDir;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_git_repo_with_commits;

fn setup_test_repo() -> Result<(TempDir, Arc<GitRepo>)> {
    let (temp_dir, git_repo) = setup_git_repo_with_commits()?;
    Ok((temp_dir, Arc::new(git_repo)))
}

#[tokio::test]
async fn test_perform_commit() -> Result<()> {
    let (temp_dir, _git_repo) = setup_test_repo()?;
    let use_gitmoji = true;
    let verify = true;

    // Create a new GitRepo for the service
    let service_repo = Arc::new(GitRepo::new(temp_dir.path())?);

    let service = GitCommitService::new(service_repo, use_gitmoji, verify);

    let result = service.perform_commit("Test commit message")?;
    println!("Perform commit result: {result:?}");

    // Verify the commit was made
    let repo = git2::Repository::open(temp_dir.path())?;
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Test commit message"
    );

    Ok(())
}

#[test]
fn initial_commit_supports_context_staging_and_statistics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo = git2::Repository::init(temp_dir.path())?;
    repo.set_head("refs/heads/new-project")?;
    repo.config()?.set_str("user.name", "First Author")?;
    repo.config()?.set_str("user.email", "first@example.com")?;
    let git_repo = Arc::new(GitRepo::new(temp_dir.path())?);
    let path = std::path::Path::new("first.txt");
    std::fs::write(temp_dir.path().join(path), "first\nsecond\n")?;
    git_repo.stage_file(path)?;

    let context = git_repo.get_git_info(&git_iris::config::Config::default())?;
    assert_eq!(context.branch, "new-project");
    assert!(context.recent_commits.is_empty());
    assert_eq!(context.staged_files.len(), 1);

    git_repo.unstage_file(path)?;
    assert!(repo.index()?.is_empty());
    assert!(temp_dir.path().join(path).is_file());
    git_repo.stage_file(path)?;
    git_repo.unstage_all()?;
    assert!(repo.index()?.is_empty());
    assert!(temp_dir.path().join(path).is_file());
    git_repo.stage_file(path)?;

    let service = GitCommitService::new(git_repo, false, false);
    let result = service.perform_commit("Initial project")?;
    let commit = repo.head()?.peel_to_commit()?;
    assert_eq!(commit.parent_count(), 0);
    assert_eq!(commit.message()?, "Initial project");
    assert_eq!(result.files_changed, 1);
    assert_eq!(result.insertions, 2);
    assert_eq!(result.deletions, 0);
    assert_eq!(
        result.new_files,
        vec![("first.txt".into(), git2::FileMode::Blob)]
    );
    Ok(())
}

#[test]
fn amend_preserves_author_and_updates_committer_and_statistics() -> Result<()> {
    let (temp_dir, git_repo) = setup_test_repo()?;
    let repo = git2::Repository::open(temp_dir.path())?;
    let original = repo.head()?.peel_to_commit()?;
    let original_author = git2::Signature::new(
        "Original Author",
        "original@example.com",
        &git2::Time::new(1_000_000, 60),
    )?;
    original.amend(Some("HEAD"), Some(&original_author), None, None, None, None)?;
    repo.config()?.set_str("user.name", "Current Committer")?;
    repo.config()?
        .set_str("user.email", "current@example.com")?;
    std::fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}\n")?;
    git_repo.stage_file(std::path::Path::new("src/main.rs"))?;

    let service = GitCommitService::new(git_repo, false, false);
    let result = service.perform_amend("Amended project")?;
    let amended = repo.head()?.peel_to_commit()?;
    assert_eq!(amended.author().name()?, "Original Author");
    assert_eq!(amended.author().email()?, "original@example.com");
    assert_eq!(amended.author().when(), original_author.when());
    assert_eq!(amended.committer().name()?, "Current Committer");
    assert_eq!(amended.parent_id(0)?, original.parent_id(0)?);
    assert_eq!(result.files_changed, 1);
    assert_eq!(result.insertions, 1);
    assert_eq!(
        result.new_files,
        vec![("src/main.rs".into(), git2::FileMode::Blob)]
    );
    Ok(())
}

#[test]
fn commit_reports_insertions_and_deletions_from_committed_tree() -> Result<()> {
    let (temp_dir, git_repo) = setup_test_repo()?;
    std::fs::write(temp_dir.path().join("README.md"), "replacement\nextra\n")?;
    git_repo.stage_file(std::path::Path::new("README.md"))?;
    let service = GitCommitService::new(git_repo, false, false);
    let result = service.perform_commit("Replace readme")?;
    assert_eq!(result.files_changed, 1);
    assert_eq!(result.insertions, 2);
    assert_eq!(result.deletions, 1);
    assert!(result.new_files.is_empty());
    Ok(())
}

#[test]
fn detached_head_is_identified_for_pr_detection() -> Result<()> {
    let (temp_dir, git_repo) = setup_test_repo()?;
    let repo = git2::Repository::open(temp_dir.path())?;
    repo.set_head_detached(repo.head()?.peel_to_commit()?.id())?;
    assert_eq!(git_repo.get_current_branch()?, "HEAD detached");
    Ok(())
}
