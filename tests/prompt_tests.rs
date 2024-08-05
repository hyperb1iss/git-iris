use git_iris::config::Config;
use git_iris::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use git_iris::prompt::{create_prompt, create_user_prompt};

fn create_mock_commit_context() -> CommitContext {
    CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![RecentCommit {
            hash: "abcdef1".to_string(),
            message: "Initial commit".to_string(),
            author: "Test User".to_string(),
            timestamp: "1234567890".to_string(),
        }],
        staged_files: vec![StagedFile {
            path: "file1.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "- old line\n+ new line".to_string(),
            analysis: vec!["Modified function: main".to_string()],
            content_excluded: false,
        }],
        unstaged_files: vec!["unstaged_file.txt".to_string()],
        project_metadata: ProjectMetadata {
            language: Some("Rust".to_string()),
            framework: None,
            dependencies: vec![],
            version: None,
            build_system: None,
            test_framework: None,
            plugins: vec![],
        },
    }
}

#[test]
fn test_create_prompt_basic() {
    let commit_context = create_mock_commit_context();
    let config = Config::default();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("Initial commit"));
    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("unstaged_file.txt"));
}

#[test]
fn test_create_prompt_with_staged_files() {
    let commit_context = create_mock_commit_context();
    let config = Config::default();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
}

#[test]
fn test_create_prompt_with_gitmoji() {
    let commit_context = create_mock_commit_context();
    let mut config = Config::default();
    config.use_gitmoji = true;

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("✨ - :feat: - Introduce new features"));
    assert!(prompt.contains("🐛 - :fix: - Fix a bug"));
    assert!(prompt.contains("📝 - :docs: - Add or update documentation"));
    assert!(prompt.contains("💄 - :style: - Add or update the UI and style files"));
    assert!(prompt.contains("♻️ - :refactor: - Refactor code"));
    assert!(prompt.contains("✅ - :test: - Add or update tests"));
    assert!(prompt.contains("🔨 - :chore: - Other changes that don't modify src or test files"));
}

#[test]
fn test_create_prompt_with_custom_instructions() {
    let commit_context = create_mock_commit_context();
    let mut config = Config::default();
    config.instructions = "Always mention the ticket number".to_string();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("Always mention the ticket number"));
}

#[test]
fn test_create_prompt_verbose() {
    let commit_context = create_mock_commit_context();
    let config = Config::default();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("Detailed changes"));
}

#[test]
fn test_create_user_prompt() {
    let commit_context = create_mock_commit_context();
    let prompt = create_user_prompt(&commit_context).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("Initial commit"));
    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
    assert!(prompt.contains("unstaged_file.txt"));
}

#[test]
fn test_create_prompt_with_multiple_staged_files() {
    let mut commit_context = create_mock_commit_context();
    commit_context.staged_files.push(StagedFile {
        path: "file2.rs".to_string(),
        change_type: ChangeType::Added,
        diff: "+ new file content".to_string(),
        analysis: vec!["New function: helper".to_string()],
        content_excluded: false,
    });

    let config = Config::default();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("file2.rs"));
    assert!(prompt.contains("Added"));
    assert!(prompt.contains("- old line\n+ new line"));
    assert!(prompt.contains("+ new file content"));
}

#[test]
fn test_create_prompt_with_project_metadata() {
    let mut commit_context = create_mock_commit_context();
    commit_context.project_metadata = ProjectMetadata {
        language: Some("Rust".to_string()),
        framework: Some("Rocket".to_string()),
        dependencies: vec!["serde".to_string(), "tokio".to_string()],
        version: None,
        build_system: None,
        test_framework: None,
        plugins: vec![],
    };

    let config = Config::default();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("Language: Rust"));
    assert!(prompt.contains("Framework: Rocket"));
    assert!(prompt.contains("Dependencies: serde, tokio"));
}

#[test]
fn test_create_prompt_with_file_analysis() {
    let mut commit_context = create_mock_commit_context();
    commit_context.staged_files[0].analysis = vec![
        "Modified function: main".to_string(),
        "Added new struct: User".to_string(),
    ];

    let config = Config::default();

    let prompt = create_prompt(&commit_context, &config).unwrap();

    assert!(prompt.contains("Modified function: main"));
    assert!(prompt.contains("Added new struct: User"));
}

