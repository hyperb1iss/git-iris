use git2::Repository;
use git_iris::git::{get_git_info, commit};
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

    let git_info = get_git_info(temp_dir.path()).unwrap();

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
    let updated_git_info = get_git_info(temp_dir.path()).unwrap();

    // Test staged files
    assert_eq!(updated_git_info.staged_files.len(), 1);
    assert!(updated_git_info.staged_files.contains_key("new_file.txt"));

    // Test unstaged files
    assert_eq!(updated_git_info.unstaged_files.len(), 1);
    assert_eq!(updated_git_info.unstaged_files[0], "unstaged.txt");
}

#[test]
fn test_commit() {
    let temp_dir = setup_git_repo();

    // Create and stage a new file
    let new_file_path = temp_dir.path().join("commit_test.txt");
    fs::write(&new_file_path, "Commit test content").unwrap();
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("commit_test.txt")).unwrap();
    index.write().unwrap();

    // Perform commit
    let result = commit(temp_dir.path(), "Test commit message");
    assert!(result.is_ok());

    // Verify commit
    let git_info = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(git_info.recent_commits.len(), 2);
    assert!(git_info.recent_commits[0].contains("Test commit message"));
}

#[test]
fn test_multiple_staged_files() {
    let temp_dir = setup_git_repo();

    // Create and stage multiple files
    for i in 1..=3 {
        let file_path = temp_dir.path().join(format!("file{}.txt", i));
        fs::write(&file_path, format!("Content {}", i)).unwrap();
        let repo = Repository::open(temp_dir.path()).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(&format!("file{}.txt", i))).unwrap();
        index.write().unwrap();
    }

    let git_info = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(git_info.staged_files.len(), 3);
    for i in 1..=3 {
        assert!(git_info.staged_files.contains_key(&format!("file{}.txt", i)));
    }
}

#[test]
fn test_modified_file() {
    let temp_dir = setup_git_repo();

    // Modify the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Modified content").unwrap();
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("initial.txt")).unwrap();
    index.write().unwrap();

    let git_info = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(git_info.staged_files.len(), 1);
    assert!(git_info.staged_files.contains_key("initial.txt"));
    assert_eq!(git_info.staged_files["initial.txt"].status, "M");
}

#[test]
fn test_deleted_file() {
    let temp_dir = setup_git_repo();

    // Delete the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::remove_file(&initial_file_path).unwrap();
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("initial.txt")).unwrap();
    index.write().unwrap();

    let git_info = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(git_info.staged_files.len(), 1);
    assert!(git_info.staged_files.contains_key("initial.txt"));
    assert_eq!(git_info.staged_files["initial.txt"].status, "D");
}