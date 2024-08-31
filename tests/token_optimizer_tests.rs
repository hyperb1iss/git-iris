use git_iris::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use git_iris::token_optimizer::TokenOptimizer;

const DEBUG: bool = false;

// Helper function to create a test context
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
                content: Some("Full content of file1.rs".to_string()),
            },
            StagedFile {
                path: "file2.rs".to_string(),
                change_type: ChangeType::Added,
                diff: "+ New file content".to_string(),
                analysis: vec!["Added new struct: TestStruct".to_string()],
                content_excluded: false,
                content: Some("Full content of file2.rs".to_string()),
            },
        ],
        project_metadata: ProjectMetadata {
            language: Some("Rust".to_string()),
            framework: None,
            dependencies: vec![],
            version: Some("0.1.0".to_string()),
            build_system: Some("Cargo".to_string()),
            test_framework: None,
            plugins: vec![],
        },
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
    }
}

// Test case for small token limit to ensure diffs and commits are prioritized over full content
#[test]
fn test_token_optimizer_prioritize_diffs_and_commits() {
    let mut context = create_test_context();

    let optimizer = TokenOptimizer::new(15); // Small token limit
    println!(
        "Original token count: {}",
        count_total_tokens(&context, &optimizer)
    );

    optimizer.optimize_context(&mut context);

    print_debug_info(&context, &optimizer);

    let total_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        total_tokens <= 15,
        "Total tokens ({total_tokens}) exceeds limit of 15"
    );

    // File diffs should be fully represented or truncated last
    for file in &context.staged_files {
        assert!(
            optimizer.count_tokens(&file.diff) <= 15,
            "File diff should be within token limit"
        );
    }

    // Check commit messages, ensuring that only the last commit might be truncated
    for (i, commit) in context.recent_commits.iter().enumerate() {
        let commit_tokens = optimizer.count_tokens(&commit.message);
        if i == context.recent_commits.len() - 1 {
            // Only the last commit should potentially be truncated
            assert!(
                commit.message.ends_with('…') || commit_tokens <= 1,
                "Last commit message should be truncated or be at most 1 character long"
            );
        } else {
            // Earlier commits should not be truncated unless there's a severe limit
            assert!(
                commit_tokens <= 15,
                "Earlier commit messages should fit within token limits"
            );
        }
    }

    // Full file content should be the lowest priority and truncated first if needed
    for file in &context.staged_files {
        if let Some(content) = &file.content {
            assert!(
                content.ends_with('…') || optimizer.count_tokens(content) <= 1,
                "Full file content should be truncated or be at most 1 character long"
            );
        }
    }
}

// Test case for large token limit to ensure no content is truncated
#[test]
fn test_token_optimizer_large_limit_with_full_content() {
    let mut context = create_test_context();
    let optimizer = TokenOptimizer::new(1000); // Large token limit

    optimizer.optimize_context(&mut context);

    let total_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        total_tokens <= 1000,
        "Total tokens ({total_tokens}) exceeds limit of 1000"
    );

    // No truncation should occur, especially in file diffs and full content
    for file in &context.staged_files {
        assert!(
            !file.diff.ends_with('…'),
            "File diff should not be truncated"
        );
        if let Some(content) = &file.content {
            assert!(
                !content.ends_with('…'),
                "Full file content should not be truncated"
            );
        }
    }

    for commit in &context.recent_commits {
        assert!(
            !commit.message.ends_with('…'),
            "Commit message should not be truncated"
        );
    }
}

// Helper function to print debug information
fn print_debug_info(context: &CommitContext, optimizer: &TokenOptimizer) {
    if !DEBUG {
        return;
    }
    println!("Commits: {}", context.recent_commits.len());
    for (i, commit) in context.recent_commits.iter().enumerate() {
        let tokens = optimizer.count_tokens(&commit.message);
        println!("Commit {}: '{}' ({} tokens)", i, commit.message, tokens);
    }
    println!("Staged files: {}", context.staged_files.len());
    for (i, file) in context.staged_files.iter().enumerate() {
        let diff_tokens = optimizer.count_tokens(&file.diff);
        println!(
            "Staged file {}: '{}' ({} tokens)",
            i, file.diff, diff_tokens
        );
        if let Some(content) = &file.content {
            let content_tokens = optimizer.count_tokens(content);
            println!("Full content {i}: '{content}' ({content_tokens} tokens)");
        }
    }
}

#[test]
fn test_token_optimizer_realistic_limit() {
    let mut context = create_test_context_with_large_data(); // Function that creates the test data
    let optimizer = TokenOptimizer::new(2000); // Realistic token limit

    println!(
        "Test token count: {}",
        count_total_tokens(&context, &optimizer)
    );

    // Apply the optimizer to bring the token count within the limit
    optimizer.optimize_context(&mut context);

    // Debugging print to verify the final token count
    let total_tokens = count_total_tokens(&context, &optimizer);
    println!("Total tokens after optimization: {total_tokens}");

    // Assert that the total tokens do not exceed the limit
    assert!(
        total_tokens <= 2000,
        "Total tokens ({total_tokens}) exceeds limit of 2000"
    );

    // Verify that the diffs are prioritized and potentially truncated last
    for file in &context.staged_files {
        let diff_tokens = optimizer.count_tokens(&file.diff);
        if let Some(content) = &file.content {
            let content_tokens = optimizer.count_tokens(content);
            assert!(
                content_tokens <= 2000 - diff_tokens,
                "Full file content should be truncated first if necessary"
            );
        }
        assert!(
            diff_tokens <= 2000,
            "File diff should be within the token limit after truncation"
        );
    }

    // Check that commit messages are truncated if necessary, prioritizing diffs
    for (i, commit) in context.recent_commits.iter().enumerate() {
        let commit_tokens = optimizer.count_tokens(&commit.message);
        if i == context.recent_commits.len() - 1 {
            assert!(
                commit.message.ends_with('…') || commit_tokens <= 1,
                "Last commit message should be truncated if necessary"
            );
        } else {
            assert!(
                commit_tokens <= 2000,
                "Earlier commit messages should fit within token limits"
            );
        }
    }
}

// Helper function to create realistic large test data
fn create_test_context_with_large_data() -> CommitContext {
    let large_diff = "- Old line\n+ New line\n".repeat(200); // 200 repetitions to simulate a large diff
    let large_content = "Full content of the file\n".repeat(200); // Large full file content
    let large_commit_message =
        "Implemented a large feature that touches many parts of the codebase".repeat(20); // Large commit message

    CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![
            RecentCommit {
                hash: "abc123".to_string(),
                message: large_commit_message.clone(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "def456".to_string(),
                message: large_commit_message.clone(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-02 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "ghi789".to_string(),
                message: large_commit_message,
                author: "Test Author".to_string(),
                timestamp: "2023-01-03 00:00:00".to_string(),
            },
        ],
        staged_files: vec![
            StagedFile {
                path: "file1.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: large_diff.clone(),
                analysis: vec!["Modified function: test_function".to_string()],
                content_excluded: false,
                content: Some(large_content.clone()),
            },
            StagedFile {
                path: "file2.rs".to_string(),
                change_type: ChangeType::Added,
                diff: large_diff,
                analysis: vec!["Added new struct: TestStruct".to_string()],
                content_excluded: false,
                content: Some(large_content),
            },
        ],
        project_metadata: ProjectMetadata {
            language: Some("Rust".to_string()),
            framework: None,
            dependencies: vec![],
            version: Some("0.1.0".to_string()),
            build_system: Some("Cargo".to_string()),
            test_framework: None,
            plugins: vec![],
        },
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
    }
}

// Helper function to count total tokens
fn count_total_tokens(context: &CommitContext, optimizer: &TokenOptimizer) -> usize {
    let commit_tokens: usize = context
        .recent_commits
        .iter()
        .map(|c| optimizer.count_tokens(&c.message))
        .sum();
    let staged_tokens: usize = context
        .staged_files
        .iter()
        .map(|f| {
            optimizer.count_tokens(&f.diff)
                + f.content.as_ref().map_or(0, |c| optimizer.count_tokens(c))
        })
        .sum();
    commit_tokens + staged_tokens
}
