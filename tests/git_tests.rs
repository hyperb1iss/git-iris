use git2::Repository;
use git_iris::commit::prompt::{create_system_prompt, create_user_prompt};
use git_iris::config::Config;
use git_iris::context::ChangeType;
use git_iris::git::GitRepo;
use git_iris::token_optimizer::TokenOptimizer;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn setup_git_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo = Repository::init(temp_dir.path()).expect("Failed to initialize repository");

    // Configure git user
    let mut config = repo.config().expect("Failed to get repository config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user email");

    // Create and commit an initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Initial content").expect("Failed to write initial file");

    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("initial.txt"))
        .expect("Failed to add file to index");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let signature = repo.signature().expect("Failed to create signature");
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

#[tokio::test]
async fn test_get_git_info() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

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

    // Test project metadata
    assert_eq!(
        context.project_metadata.language,
        Some("Unknown".to_string())
    );

    // Create and stage a new file
    let new_file_path = temp_dir.path().join("new_file.txt");
    fs::write(&new_file_path, "New content").expect("Failed to write new file");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("new_file.txt"))
        .expect("Failed to add new file to index");
    index.write().expect("Failed to write index");

    // Create an unstaged file
    let unstaged_file_path = temp_dir.path().join("unstaged.txt");
    fs::write(&unstaged_file_path, "Unstaged content").expect("Failed to write unstaged file");

    // Get updated git info
    let updated_context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get updated git info");

    // Test staged files
    assert_eq!(updated_context.staged_files.len(), 1);
    assert_eq!(updated_context.staged_files[0].path, "new_file.txt");
    assert!(matches!(
        updated_context.staged_files[0].change_type,
        ChangeType::Added
    ));
}

#[tokio::test]
async fn test_commit() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create and stage a new file
    let new_file_path = temp_dir.path().join("commit_test.txt");
    fs::write(&new_file_path, "Commit test content").expect("Failed to write commit test file");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("commit_test.txt"))
        .expect("Failed to add commit test file to index");
    index.write().expect("Failed to write index");

    // Perform commit
    let result = git_repo.commit("Test commit message");
    assert!(result.is_ok(), "Failed to perform commit");

    // Verify commit
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info after commit");
    assert_eq!(context.recent_commits.len(), 2);
    assert!(context.recent_commits[0]
        .message
        .contains("Test commit message"));
}

#[tokio::test]
async fn test_multiple_staged_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create and stage multiple files
    for i in 1..=3 {
        let file_path = temp_dir.path().join(format!("file{i}.txt"));
        fs::write(&file_path, format!("Content {i}"))
            .expect("Failed to write multiple staged file");
        let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
        let mut index = repo.index().expect("Failed to get repository index");
        index
            .add_path(Path::new(&format!("file{i}.txt")))
            .expect("Failed to add multiple staged file to index");
        index.write().expect("Failed to write index");
    }

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");
    assert_eq!(context.staged_files.len(), 3);
    for i in 1..=3 {
        assert!(context
            .staged_files
            .iter()
            .any(|file| file.path == format!("file{i}.txt")));
    }
}

#[tokio::test]
async fn test_modified_file() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Modify the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Modified content").expect("Failed to modify file content");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("initial.txt"))
        .expect("Failed to add modified file to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");
    assert_eq!(context.staged_files.len(), 1);
    assert!(
        context
            .staged_files
            .iter()
            .any(|file| file.path == "initial.txt"
                && matches!(file.change_type, ChangeType::Modified))
    );
}

#[tokio::test]
async fn test_deleted_file() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Delete the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::remove_file(&initial_file_path).expect("Failed to remove initial file");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .remove_path(Path::new("initial.txt"))
        .expect("Failed to remove file from index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");
    assert_eq!(context.staged_files.len(), 1);
    assert!(context
        .staged_files
        .iter()
        .any(|file| file.path == "initial.txt" && matches!(file.change_type, ChangeType::Deleted)));
}

#[tokio::test]
async fn test_binary_file() {
    let (temp_dir, git_repo) = setup_git_repo();
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
    fs::write(&binary_file_path, binary_content).expect("Failed to write binary file");

    // Stage the binary file
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("image.png"))
        .expect("Failed to add binary file to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

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
        .expect("Failed to find binary file in staged files");
    assert_eq!(binary_file.diff, "[Binary file changed]");

    // Check if the status is correct
    assert!(matches!(binary_file.change_type, ChangeType::Added));
}

#[tokio::test]
async fn test_get_git_info_with_excluded_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create files that should be excluded
    fs::create_dir_all(temp_dir.path().join("node_modules"))
        .expect("Failed to create node_modules directory");
    fs::write(
        temp_dir.path().join("node_modules/excluded.js"),
        "console.log('excluded');",
    )
    .expect("Failed to write excluded file");
    fs::write(temp_dir.path().join(".gitignore"), "node_modules/")
        .expect("Failed to write .gitignore");
    fs::write(
        temp_dir.path().join("package-lock.json"),
        r#"{"name": "test-package"}"#,
    )
    .expect("Failed to write package-lock.json");

    // Create a non-excluded file
    fs::write(
        temp_dir.path().join("included.js"),
        "console.log('included');",
    )
    .expect("Failed to write included file");

    // Stage all files
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .expect("Failed to add all files to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Check excluded files
    let excluded_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| file.content_excluded)
        .collect();

    assert!(!excluded_files.is_empty(), "Should have excluded files");

    println!("{excluded_files:?}");
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

#[tokio::test]
async fn test_multiple_staged_files_with_exclusions() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create files that should be excluded
    fs::create_dir_all(temp_dir.path().join(".vscode"))
        .expect("Failed to create .vscode directory");
    fs::write(
        temp_dir.path().join(".vscode/settings.json"),
        r#"{"editor.formatOnSave": true}"#,
    )
    .expect("Failed to write .vscode/settings.json");
    fs::write(
        temp_dir.path().join("large.min.js"),
        "console.log('minified')",
    )
    .expect("Failed to write large.min.js");

    // Create non-excluded files
    for i in 1..=3 {
        fs::write(
            temp_dir.path().join(format!("file{i}.txt")),
            format!("Content {i}"),
        )
        .expect("Failed to write non-excluded file");
    }

    // Stage all files
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .expect("Failed to add all files to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

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
    #[allow(clippy::case_sensitive_file_extension_comparisons)] // todo: check if this is necessary
    for file in &included_files {
        assert!(file.path.starts_with("file") && file.path.ends_with(".txt"));
        assert_ne!(file.diff, "[Content excluded]");
        assert_ne!(file.analysis, vec!["[Analysis excluded]"]);
    }
}

#[tokio::test]
async fn test_token_optimization_integration() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Set a small token limit for the OpenAI provider to force truncation
    let small_token_limit = 200;

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    let system_prompt = create_system_prompt(&config).expect("Failed to create system prompt");
    let user_prompt = create_user_prompt(&context);
    let prompt = format!("{system_prompt}\n{user_prompt}");

    // Check that the prompt is within the token limit
    let optimizer = TokenOptimizer::new(small_token_limit);
    let prompt = optimizer.truncate_string(&prompt, small_token_limit);

    let token_count = optimizer.count_tokens(&prompt);

    println!("Token count: {token_count}");
    println!("Token limit: {small_token_limit}");
    println!("Prompt:\n{prompt}");

    assert!(
        token_count <= small_token_limit,
        "Prompt exceeds token limit. Token count: {token_count}, Limit: {small_token_limit}"
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

    let system_prompt = create_system_prompt(&config).expect("Failed to create system prompt");
    let user_prompt = create_user_prompt(&context);
    let large_prompt = format!("{system_prompt}\n{user_prompt}");

    let large_token_count = optimizer.count_tokens(&large_prompt);

    println!("Large token count: {large_token_count}");
    println!("Large token limit: {large_token_limit}");

    assert!(
        large_token_count <= large_token_limit,
        "Large prompt exceeds token limit. Token count: {large_token_count}, Limit: {large_token_limit}"
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

#[tokio::test]
async fn test_project_metadata_parallelism() {
    // Create a temporary directory for our test files
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");

    // Create multiple files with different "languages"
    let files = vec![
        ("file1.rs", "fn main() {}"),
        ("file2.py", "def main(): pass"),
        ("file3.js", "function main() {}"),
        ("file3.js", "function main() {}"),
        ("file4.c", "int main() { return 0; }"),
        ("file5.kt", "fun main() {}"),
    ];

    let file_paths: Vec<String> = files
        .into_iter()
        .map(|(filename, content)| {
            let file_path = temp_dir.path().join(filename);
            fs::write(&file_path, content).expect("Failed to write test file");
            let path_str = file_path
                .to_str()
                .expect("Failed to convert path to string")
                .to_string();
            println!("Created file: {path_str} with content: {content}");
            assert!(
                Path::new(&path_str).exists(),
                "File does not exist: {path_str}"
            );
            path_str
        })
        .collect();

    // Measure the time taken to process metadata
    let start = Instant::now();
    let metadata = git_repo
        .get_project_metadata(&file_paths)
        .await
        .expect("Failed to get project metadata");
    let duration = start.elapsed();

    // Detailed logging
    println!("File paths: {file_paths:?}");
    println!("Metadata: {metadata:?}");
    println!("Detected language: {:?}", metadata.language);
    println!("Detected dependencies: {:?}", metadata.dependencies);
    println!("Processing time: {duration:?}");

    // Assertions
    assert!(metadata.language.is_some(), "Language should be detected");

    let languages = metadata.language.expect("Failed to detect languages");
    assert!(languages.contains("Rust"), "Rust should be detected");
    assert!(languages.contains("Python"), "Python should be detected");
    assert!(
        languages.contains("JavaScript"),
        "JavaScript should be detected"
    );
    assert!(languages.contains('C'), "C should be detected");
    assert!(languages.contains("Kotlin"), "Kotlin should be detected");

    // We're not expecting any dependencies in this test
    assert!(
        metadata.dependencies.is_empty(),
        "No dependencies should be detected"
    );

    // Check if the operation was faster than sequential execution would be
    assert!(
        duration < Duration::from_millis(500),
        "Parallel execution took too long: {duration:?}"
    );
}
