use git_iris::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use git_iris::token_optimizer::TokenOptimizer;

fn create_test_context() -> CommitContext {
    CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![
            RecentCommit {
                hash: "abc123".to_string(),
                message: "Initial commit".to_string(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "def456".to_string(),
                message: "Add new feature".to_string(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-02 00:00:00".to_string(),
            },
        ],
        staged_files: vec![
            StagedFile {
                path: "file1.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- Old line\n+ New line".to_string(),
                analysis: vec!["Modified function: test_function".to_string()],
                content_excluded: false,
            },
            StagedFile {
                path: "file2.rs".to_string(),
                change_type: ChangeType::Added,
                diff: "+ New file content".to_string(),
                analysis: vec!["Added new struct: TestStruct".to_string()],
                content_excluded: false,
            },
        ],
        unstaged_files: vec!["unstaged1.rs".to_string(), "unstaged2.rs".to_string()],
        project_metadata: ProjectMetadata {
            language: Some("Rust".to_string()),
            framework: None,
            dependencies: vec![],
            version: Some("0.1.0".to_string()),
            build_system: Some("Cargo".to_string()),
            test_framework: None,
            plugins: vec![],
        },
    }
}

fn print_debug_info(context: &CommitContext, optimizer: &TokenOptimizer) {
    println!("Commits: {}", context.recent_commits.len());
    for (i, commit) in context.recent_commits.iter().enumerate() {
        println!(
            "Commit {}: '{}' ({} tokens)",
            i,
            commit.message,
            optimizer.count_tokens(&commit.message)
        );
    }
    println!("Staged files: {}", context.staged_files.len());
    for (i, file) in context.staged_files.iter().enumerate() {
        println!(
            "Staged file {}: '{}' ({} tokens)",
            i,
            file.diff,
            optimizer.count_tokens(&file.diff)
        );
    }
    println!("Unstaged files: {}", context.unstaged_files.len());
    for (i, file) in context.unstaged_files.iter().enumerate() {
        println!(
            "Unstaged file {}: '{}' ({} tokens)",
            i,
            file,
            optimizer.count_tokens(file)
        );
    }
}

fn count_total_tokens(context: &CommitContext, optimizer: &TokenOptimizer) -> usize {
    let commit_tokens: usize = context
        .recent_commits
        .iter()
        .map(|c| optimizer.count_tokens(&c.message))
        .sum();
    let staged_tokens: usize = context
        .staged_files
        .iter()
        .map(|f| optimizer.count_tokens(&f.diff))
        .sum();
    let unstaged_tokens: usize = context
        .unstaged_files
        .iter()
        .map(|f| optimizer.count_tokens(f))
        .sum();
    commit_tokens + staged_tokens + unstaged_tokens
}

#[test]
fn test_token_optimizer() {
    let mut context = create_test_context();
    let optimizer = TokenOptimizer::new(20); // Very small token limit for testing

    optimizer.optimize_context(&mut context);

    print_debug_info(&context, &optimizer);

    // Check if content was truncated
    assert!(
        context.recent_commits.len() <= 2,
        "Expected 2 or fewer commits, got {}",
        context.recent_commits.len()
    );
    assert!(
        context.staged_files.len() <= 2,
        "Expected 2 or fewer staged files, got {}",
        context.staged_files.len()
    );
    assert!(
        context.unstaged_files.len() <= 2,
        "Expected 2 or fewer unstaged files, got {}",
        context.unstaged_files.len()
    );

    // Check total tokens
    let total_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        total_tokens <= 20,
        "Total tokens ({}) exceeds limit of 20",
        total_tokens
    );

    // Check that all strings end with '…' if they were truncated
    for commit in &context.recent_commits {
        if optimizer.count_tokens(&commit.message) < optimizer.count_tokens("Initial commit") {
            assert!(
                commit.message.ends_with('…'),
                "Truncated commit message should end with '…'"
            );
        }
    }
    for file in &context.staged_files {
        if optimizer.count_tokens(&file.diff) < optimizer.count_tokens("- Old line\n+ New line") {
            assert!(
                file.diff.ends_with('…'),
                "Truncated diff should end with '…'"
            );
        }
    }
    for file in &context.unstaged_files {
        if optimizer.count_tokens(file) < optimizer.count_tokens("unstaged1.rs") {
            assert!(
                file.ends_with('…'),
                "Truncated unstaged file name should end with '…'"
            );
        }
    }
}

#[test]
fn test_token_optimizer_very_small_limit() {
    let mut context = create_test_context();
    let optimizer = TokenOptimizer::new(10); // Very small token limit

    optimizer.optimize_context(&mut context);

    print_debug_info(&context, &optimizer);

    // Check total tokens
    let total_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        total_tokens <= 10,
        "Total tokens ({}) exceeds limit of 10",
        total_tokens
    );

    // Check that all remaining strings end with '…' if they were truncated
    for commit in &context.recent_commits {
        if optimizer.count_tokens(&commit.message) < optimizer.count_tokens("Initial commit") {
            assert!(
                commit.message.ends_with('…'),
                "Truncated commit message should end with '…'"
            );
        }
    }
    for file in &context.staged_files {
        if optimizer.count_tokens(&file.diff) < optimizer.count_tokens("- Old line\n+ New line") {
            assert!(
                file.diff.ends_with('…'),
                "Truncated diff should end with '…'"
            );
        }
    }
    for file in &context.unstaged_files {
        if optimizer.count_tokens(file) < optimizer.count_tokens("unstaged1.rs") {
            assert!(
                file.ends_with('…'),
                "Truncated unstaged file name should end with '…'"
            );
        }
    }
}

#[test]
fn test_token_optimizer_large_limit() {
    let mut context = create_test_context();
    let optimizer = TokenOptimizer::new(10000); // Large token limit

    let original_commits = context.recent_commits.len();
    let original_staged = context.staged_files.len();
    let original_unstaged = context.unstaged_files.len();

    optimizer.optimize_context(&mut context);

    // Check that nothing was truncated
    assert_eq!(context.recent_commits.len(), original_commits);
    assert_eq!(context.staged_files.len(), original_staged);
    assert_eq!(context.unstaged_files.len(), original_unstaged);

    // Check that no strings were modified
    for (original, optimized) in create_test_context()
        .recent_commits
        .iter()
        .zip(context.recent_commits.iter())
    {
        assert_eq!(original.message, optimized.message);
    }
    for (original, optimized) in create_test_context()
        .staged_files
        .iter()
        .zip(context.staged_files.iter())
    {
        assert_eq!(original.diff, optimized.diff);
    }
    assert_eq!(create_test_context().unstaged_files, context.unstaged_files);
}
