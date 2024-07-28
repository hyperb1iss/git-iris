use git2::Repository;
use git_iris::git::get_git_info;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn setup_git_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo = Repository::init(temp_dir.path()).unwrap();

    // Configure git user
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // Create and commit an initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Initial content").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("initial.txt")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = repo.signature().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    temp_dir
}

#[test]
fn test_get_git_info() {
    let temp_dir = setup_git_repo();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let git_info = get_git_info().unwrap();

    // Test branch name
    assert!(
        git_info.branch == "main" || git_info.branch == "master",
        "Branch should be 'main' or 'master', but got '{}'",
        git_info.branch
    );

    // Test recent commits
    assert_eq!(git_info.recent_commits.len(), 1);
    assert!(git_info.recent_commits[0].contains("Initial commit"));

    // Test staged files (should be empty after commit)
    assert_eq!(git_info.staged_files.len(), 0);

    // Test unstaged files (should be empty after commit)
    assert_eq!(git_info.unstaged_files.len(), 0);

    // Test project root
    assert_eq!(git_info.project_root, temp_dir.path().to_str().unwrap());

    // Create and stage a new file
    let new_file_path = temp_dir.path().join("new_file.txt");
    fs::write(&new_file_path, "New content").unwrap();
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("new_file.txt")).unwrap();
    index.write().unwrap();

    // Create an unstaged file
    let unstaged_file_path = temp_dir.path().join("unstaged.txt");
    fs::write(&unstaged_file_path, "Unstaged content").unwrap();

    // Get updated git info
    let updated_git_info = get_git_info().unwrap();

    // Test staged files
    assert_eq!(updated_git_info.staged_files.len(), 1);
    assert!(updated_git_info.staged_files.contains_key("new_file.txt"));

    // Test unstaged files
    assert_eq!(updated_git_info.unstaged_files.len(), 1);
    assert_eq!(updated_git_info.unstaged_files[0], "unstaged.txt");
}
