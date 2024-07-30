use git2::Repository;
use git_iris::context::ChangeType;
use git_iris::git::{commit, get_git_info};
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

    let context = get_git_info(temp_dir.path()).unwrap();

    // Test branch name
    assert!(
        context.branch == "main" || context.branch == "master",
        "Branch should be 'main' or 'master', but got '{}'",
        context.branch
    );

    // Test recent commits
    assert_eq!(context.recent_commits.len(), 1);
    assert!(context.recent_commits[0].message.contains("Initial commit"));

    // Test staged files (should be empty after commit)
    assert_eq!(context.staged_files.len(), 0);

    // Test unstaged files (should be empty after commit)
    assert_eq!(context.unstaged_files.len(), 0);

    // Test project metadata
    assert_eq!(context.project_metadata.language, "Unknown");

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
    let updated_context = get_git_info(temp_dir.path()).unwrap();

    // Test staged files
    assert_eq!(updated_context.staged_files.len(), 1);
    assert_eq!(updated_context.staged_files[0].path, "new_file.txt");
    assert!(matches!(
        updated_context.staged_files[0].change_type,
        ChangeType::Added
    ));

    // Test unstaged files
    assert_eq!(updated_context.unstaged_files.len(), 1);
    assert_eq!(updated_context.unstaged_files[0], "unstaged.txt");
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
    let context = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(context.recent_commits.len(), 2);
    assert!(context.recent_commits[0]
        .message
        .contains("Test commit message"));
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
        index
            .add_path(Path::new(&format!("file{}.txt", i)))
            .unwrap();
        index.write().unwrap();
    }

    let context = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(context.staged_files.len(), 3);
    for i in 1..=3 {
        assert!(context
            .staged_files
            .iter()
            .any(|file| file.path == format!("file{}.txt", i)));
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

    let context = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(context.staged_files.len(), 1);
    assert!(
        context
            .staged_files
            .iter()
            .any(|file| file.path == "initial.txt"
                && matches!(file.change_type, ChangeType::Modified))
    );
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

    let context = get_git_info(temp_dir.path()).unwrap();
    assert_eq!(context.staged_files.len(), 1);
    assert!(context
        .staged_files
        .iter()
        .any(|file| file.path == "initial.txt" && matches!(file.change_type, ChangeType::Deleted)));
}

#[test]
fn test_binary_file() {
    let temp_dir = setup_git_repo();

    // Create a binary file (a simple PNG file)
    let binary_file_path = temp_dir.path().join("image.png");
    let binary_content = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    fs::write(&binary_file_path, &binary_content).unwrap();

    // Stage the binary file
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("image.png")).unwrap();
    index.write().unwrap();

    let context = get_git_info(temp_dir.path()).unwrap();

    // Check if the binary file is in staged files
    assert!(context
        .staged_files
        .iter()
        .any(|file| file.path == "image.png"));

    // Check if the diff for the binary file is "[Binary file changed]"
    let binary_file = context
        .staged_files
        .iter()
        .find(|file| file.path == "image.png")
        .unwrap();
    assert_eq!(binary_file.diff, "[Binary file changed]");

    // Check if the status is correct
    assert!(matches!(binary_file.change_type, ChangeType::Added));
}
