use git_iris::changes::change_analyzer::{AnalyzedChange, FileChange};
use git_iris::changes::models::ChangeMetrics;
use git_iris::changes::models::ChangelogType;
use git_iris::changes::prompt::{
    create_changelog_system_prompt, create_changelog_user_prompt,
    create_release_notes_system_prompt, create_release_notes_user_prompt,
};
use git_iris::common::DetailLevel;
use git_iris::config::Config;
use git_iris::context::ChangeType;

/// Creates a mock configuration for testing
fn create_mock_config() -> Config {
    Config {
        use_gitmoji: true,
        instructions: "Always mention performance impacts".to_string(),
        ..Default::default()
    }
}

/// Creates a mock analyzed change for testing
fn create_mock_analyzed_change() -> AnalyzedChange {
    AnalyzedChange {
        commit_hash: "abcdef123456".to_string(),
        commit_message: "Add new feature".to_string(),
        author: "Jane Doe".to_string(),
        file_changes: vec![FileChange {
            old_path: "src/old.rs".to_string(),
            new_path: "src/new.rs".to_string(),
            change_type: ChangeType::Modified,
            analysis: vec!["Modified function: process_data".to_string()],
        }],
        metrics: ChangeMetrics {
            total_commits: 1,
            files_changed: 1,
            insertions: 15,
            deletions: 5,
            total_lines_changed: 20,
        },
        impact_score: 0.75,
        change_type: ChangelogType::Added,
        is_breaking_change: false,
        associated_issues: vec!["#123".to_string()],
        pull_request: Some("PR #456".to_string()),
    }
}

/// Creates mock total metrics for testing
fn create_mock_total_metrics() -> ChangeMetrics {
    ChangeMetrics {
        total_commits: 5,
        files_changed: 10,
        insertions: 100,
        deletions: 50,
        total_lines_changed: 150,
    }
}

#[test]
fn test_create_changelog_system_prompt() {
    let config = create_mock_config();
    let prompt = create_changelog_system_prompt(&config);

    // Assert that the prompt contains key instructions and elements
    assert!(prompt.contains("You are an AI assistant specialized in generating clear, concise, and informative changelogs"));
    assert!(prompt.contains("include tasteful, appropriate, and intelligent use of emojis"));
    assert!(prompt.contains("Always mention performance impacts"));
    assert!(
        prompt.contains("Ensure that your response is a valid JSON object matching this structure")
    );
    assert!(prompt.contains("ChangelogResponse"));
    assert!(prompt.contains("sections"));
    assert!(prompt.contains("breaking_changes"));
    assert!(prompt.contains("metrics"));
}

#[test]
fn test_create_changelog_user_prompt() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let readme_summary = Some("This project is a fantastic tool for managing workflows.");

    // Test Minimal detail level
    let minimal_prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Minimal,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );

    assert!(minimal_prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(minimal_prompt.contains("Overall Changes:"));
    assert!(minimal_prompt.contains("Total commits: 5"));
    assert!(minimal_prompt.contains("Files changed: 10"));
    assert!(minimal_prompt.contains("Total lines changed: 150"));
    assert!(minimal_prompt.contains("Insertions: 100"));
    assert!(minimal_prompt.contains("Deletions: 50"));
    assert!(minimal_prompt.contains("Commit: abcdef123456"));
    assert!(minimal_prompt.contains("Author: Jane Doe"));
    assert!(minimal_prompt.contains("Message: Add new feature"));
    assert!(minimal_prompt.contains("Type: Added"));
    assert!(minimal_prompt.contains("Breaking Change: false"));
    assert!(minimal_prompt.contains("Associated Issues: #123"));
    assert!(minimal_prompt.contains("Pull Request: PR #456"));
    assert!(minimal_prompt.contains("Impact score: 0.75"));
    assert!(!minimal_prompt.contains("File changes:"));
    assert!(minimal_prompt.contains("Project README Summary:"));
    assert!(minimal_prompt.contains("This project is a fantastic tool for managing workflows."));
    assert!(minimal_prompt.contains("Please generate a concise changelog"));

    // Test Standard detail level
    let standard_prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(standard_prompt.contains("File changes:"));
    assert!(standard_prompt.contains("src/new.rs (Modified)"));
    assert!(standard_prompt.contains("Please generate a comprehensive changelog"));

    // Test Detailed detail level
    let detailed_prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Detailed,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(detailed_prompt.contains("File changes:"));
    assert!(detailed_prompt.contains("src/new.rs (Modified)"));
    assert!(detailed_prompt.contains("Modified function: process_data"));
    assert!(detailed_prompt.contains("Please generate a highly detailed changelog"));
}

#[test]
fn test_create_release_notes_system_prompt() {
    let config = create_mock_config();
    let prompt = create_release_notes_system_prompt(&config);

    // Assert that the prompt contains key instructions and elements
    assert!(prompt.contains("You are an AI assistant specialized in generating comprehensive and user-friendly release notes"));
    assert!(prompt.contains("include tasteful, appropriate, and intelligent use of emojis"));
    assert!(prompt.contains("Always mention performance impacts"));
    assert!(
        prompt.contains("Ensure that your response is a valid JSON object matching this structure")
    );
    assert!(prompt.contains("ReleaseNotesResponse"));
    assert!(prompt.contains("sections"));
    assert!(prompt.contains("breaking_changes"));
    assert!(prompt.contains("metrics"));
}

#[test]
fn test_create_release_notes_user_prompt() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let readme_summary = Some("This project is a fantastic tool for managing workflows.");

    // Test Minimal detail level
    let minimal_prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Minimal,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(minimal_prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(minimal_prompt.contains("generate concise release notes"));
    assert!(minimal_prompt.contains("Keep the release notes brief"));
    assert!(minimal_prompt.contains("Project README Summary:"));
    assert!(minimal_prompt.contains("This project is a fantastic tool for managing workflows."));

    // Test Standard detail level
    let standard_prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(standard_prompt.contains("generate comprehensive release notes"));
    assert!(standard_prompt.contains("Provide a balanced overview"));

    // Test Detailed detail level
    let detailed_prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Detailed,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(detailed_prompt.contains("generate highly detailed release notes"));
    assert!(detailed_prompt.contains("Include detailed explanations"));
}

#[test]
fn test_changelog_user_prompt_without_readme() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        None,
    );

    assert!(!prompt.contains("Project README Summary:"));
    assert!(prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(prompt.contains("generate a comprehensive changelog"));
}

#[test]
fn test_release_notes_user_prompt_without_readme() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        None,
    );

    assert!(!prompt.contains("Project README Summary:"));
    assert!(prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(prompt.contains("generate comprehensive release notes"));
}
