use git2::Repository;
use git_iris::config::Config;
use git_iris::context::ChangeType;
use git_iris::git::{commit, get_git_info};
use git_iris::prompt::create_prompt;
use git_iris::token_optimizer::TokenOptimizer;
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
    let config = Config::default();

    let context = get_git_info(temp_dir.path(), &config).unwrap();

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
    assert_eq!(
        context.project_metadata.language,
        Some("Unknown".to_string())
    );

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
    let updated_context = get_git_info(temp_dir.path(), &config).unwrap();

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
    let config = Config::default();

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
    let context = get_git_info(temp_dir.path(), &config).unwrap();
    assert_eq!(context.recent_commits.len(), 2);
    assert!(context.recent_commits[0]
        .message
        .contains("Test commit message"));
}

#[test]
fn test_multiple_staged_files() {
    let temp_dir = setup_git_repo();
    let config = Config::default();

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

    let context = get_git_info(temp_dir.path(), &config).unwrap();
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
    let config = Config::default();

    // Modify the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Modified content").unwrap();
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("initial.txt")).unwrap();
    index.write().unwrap();

    let context = get_git_info(temp_dir.path(), &config).unwrap();
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
    let config = Config::default();

    // Delete the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::remove_file(&initial_file_path).unwrap();
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("initial.txt")).unwrap();
    index.write().unwrap();

    let context = get_git_info(temp_dir.path(), &config).unwrap();
    assert_eq!(context.staged_files.len(), 1);
    assert!(context
        .staged_files
        .iter()
        .any(|file| file.path == "initial.txt" && matches!(file.change_type, ChangeType::Deleted)));
}

#[test]
fn test_binary_file() {
    let temp_dir = setup_git_repo();
    let config = Config::default();

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

    let context = get_git_info(temp_dir.path(), &config).unwrap();

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

#[test]
fn test_get_git_info_with_excluded_files() {
    let temp_dir = setup_git_repo();
    let config = Config::default();

    // Create files that should be excluded
    fs::create_dir_all(temp_dir.path().join("node_modules")).unwrap();
    fs::write(
        temp_dir.path().join("node_modules/excluded.js"),
        "console.log('excluded');",
    )
    .unwrap();
    fs::write(temp_dir.path().join(".gitignore"), "node_modules/").unwrap();
    fs::write(
        temp_dir.path().join("package-lock.json"),
        r#"{"name": "test-package"}"#,
    )
    .unwrap();

    // Create a non-excluded file
    fs::write(
        temp_dir.path().join("included.js"),
        "console.log('included');",
    )
    .unwrap();

    // Stage all files
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();

    let context = get_git_info(temp_dir.path(), &config).unwrap();

    // Check excluded files
    let excluded_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| file.content_excluded)
        .collect();

    assert!(!excluded_files.is_empty(), "Should have excluded files");

    println!("{:?}", excluded_files);
    assert!(excluded_files
        .iter()
        .any(|file| file.path == "package-lock.json"));

    for file in &excluded_files {
        assert_eq!(file.diff, "[Content excluded]");
        assert_eq!(file.analysis, vec!["[Analysis excluded]"]);
    }

    // Check included file
    let included_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| !file.content_excluded)
        .collect();

    assert!(!included_files.is_empty(), "Should have included files");
    assert!(included_files.iter().any(|file| file.path == "included.js"));

    for file in &included_files {
        assert_ne!(file.diff, "[Content excluded]");
        assert_ne!(file.analysis, vec!["[Analysis excluded]"]);
    }
}

#[test]
fn test_multiple_staged_files_with_exclusions() {
    let temp_dir = setup_git_repo();
    let config = Config::default();

    // Create files that should be excluded
    fs::create_dir_all(temp_dir.path().join(".vscode")).unwrap();
    fs::write(
        temp_dir.path().join(".vscode/settings.json"),
        r#"{"editor.formatOnSave": true}"#,
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("large.min.js"),
        "console.log('minified')",
    )
    .unwrap();

    // Create non-excluded files
    for i in 1..=3 {
        fs::write(
            temp_dir.path().join(format!("file{}.txt", i)),
            format!("Content {}", i),
        )
        .unwrap();
    }

    // Stage all files
    let repo = Repository::open(temp_dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();

    let context = get_git_info(temp_dir.path(), &config).unwrap();

    assert_eq!(context.staged_files.len(), 5);

    let excluded_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| file.content_excluded)
        .collect();
    assert_eq!(excluded_files.len(), 2);

    let included_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| !file.content_excluded)
        .collect();
    assert_eq!(included_files.len(), 3);

    for file in &excluded_files {
        assert!(file.path.contains(".vscode") || file.path.contains(".min.js"));
        assert_eq!(file.diff, "[Content excluded]");
        assert_eq!(file.analysis, vec!["[Analysis excluded]"]);
    }

    for file in &included_files {
        assert!(file.path.starts_with("file") && file.path.ends_with(".txt"));
        assert_ne!(file.diff, "[Content excluded]");
        assert_ne!(file.analysis, vec!["[Analysis excluded]"]);
    }
}

#[test]
fn test_token_optimization_integration() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    let mut config = Config::default();
    let provider = "openai";

    // Set a small token limit for the OpenAI provider to force truncation
    let small_token_limit = 200;
    config
        .providers
        .get_mut(provider)
        .unwrap()
        .token_limit = Some(small_token_limit);

    let context = get_git_info(repo_path, &config).unwrap();

    let prompt = create_prompt(&context, &config, provider).unwrap();

    // Check that the prompt is within the token limit
    let optimizer = TokenOptimizer::new(small_token_limit);
    let token_count = optimizer.count_tokens(&prompt);

    println!("Token count: {}", token_count);
    println!("Token limit: {}", small_token_limit);
    println!("Prompt:\n{}", prompt);

    assert!(
        token_count <= small_token_limit,
        "Prompt exceeds token limit. Token count: {}, Limit: {}",
        token_count,
        small_token_limit
    );

    // Check that the prompt contains essential information
    assert!(
        prompt.contains("Git commit message"),
        "Prompt should contain instructions"
    );

    // The following assertions may fail due to truncation, so we'll make them optional
    if token_count < small_token_limit {
        assert!(
            prompt.contains("Branch:"),
            "Prompt should contain branch information"
        );
        assert!(
            prompt.contains("Recent commits:"),
            "Prompt should mention recent commits"
        );
        assert!(
            prompt.contains("Staged changes:"),
            "Prompt should mention staged changes"
        );
    }

    // Check that the prompt ends with the truncation indicator
    assert!(
        prompt.ends_with('…'),
        "Prompt should end with truncation indicator"
    );

    // Test with a larger token limit
    let large_token_limit = 5000;
    config
        .providers
        .get_mut(provider)
        .unwrap()
        .token_limit = Some(large_token_limit);

    let large_prompt = create_prompt(&context, &config, provider).unwrap();
    let large_token_count = optimizer.count_tokens(&large_prompt);

    println!("Large token count: {}", large_token_count);
    println!("Large token limit: {}", large_token_limit);

    assert!(
        large_token_count <= large_token_limit,
        "Large prompt exceeds token limit. Token count: {}, Limit: {}",
        large_token_count,
        large_token_limit
    );

    // The larger prompt should contain more information
    assert!(
        large_prompt.contains("Branch:"),
        "Large prompt should contain branch information"
    );
    assert!(
        large_prompt.contains("Recent commits:"),
        "Large prompt should mention recent commits"
    );
    assert!(
        large_prompt.contains("Staged changes:"),
        "Large prompt should mention staged changes"
    );

    // The larger prompt should not end with the truncation indicator
    assert!(
        !large_prompt.ends_with('…'),
        "Large prompt should not end with truncation indicator"
    );
}
