use git_iris::change_analyzer::{AnalyzedChange, ChangeMetrics, FileChange};
use git_iris::changelog::DetailLevel;
use git_iris::changelog_prompts::{
    create_changelog_system_prompt, create_changelog_user_prompt,
    create_release_notes_system_prompt, create_release_notes_user_prompt,
};
use git_iris::config::Config;
use git_iris::context::ChangeType;

fn create_mock_config() -> Config {
    let mut config = Config::default();
    config.use_gitmoji = true;
    config.instructions = "Always mention performance impacts".to_string();
    config
}

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
            files_changed: 1,
            insertions: 15,
            deletions: 5,
            total_lines_changed: 20,
        },
        impact_score: 0.75,
    }
}

#[test]
fn test_create_changelog_system_prompt() {
    let config = create_mock_config();
    let prompt = create_changelog_system_prompt(&config);

    assert!(prompt.contains("You are an AI assistant specialized in generating clear, concise, and informative changelogs"));
    assert!(prompt.contains("include tasteful, appropriate, and intelligent use of emojis"));
    assert!(prompt.contains("Always mention performance impacts"));
    assert!(prompt.contains("Use the provided impact scores"));
}

#[test]
fn test_create_changelog_user_prompt() {
    let changes = vec![create_mock_analyzed_change()];

    // Test Minimal detail level
    let minimal_prompt =
        create_changelog_user_prompt(&changes, DetailLevel::Minimal, "v1.0.0", "v1.1.0");
    assert!(minimal_prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(minimal_prompt.contains("Overall Changes:"));
    assert!(minimal_prompt.contains("Total commits: 1"));
    assert!(minimal_prompt.contains("Files changed: 1"));
    assert!(minimal_prompt.contains("Total lines changed: 20"));
    assert!(minimal_prompt.contains("Insertions: 15"));
    assert!(minimal_prompt.contains("Deletions: 5"));
    assert!(minimal_prompt.contains("Commit: abcdef123456"));
    assert!(minimal_prompt.contains("Impact score: 0.75"));
    assert!(!minimal_prompt.contains("File changes summary:"));

    // Test Standard detail level
    let standard_prompt =
        create_changelog_user_prompt(&changes, DetailLevel::Standard, "v1.0.0", "v1.1.0");
    assert!(standard_prompt.contains("File changes summary:"));
    assert!(standard_prompt.contains("src/new.rs (Modified)"));

    // Test Detailed detail level
    let detailed_prompt =
        create_changelog_user_prompt(&changes, DetailLevel::Detailed, "v1.0.0", "v1.1.0");
    assert!(detailed_prompt.contains("Detailed file changes:"));
    assert!(detailed_prompt.contains("Modified function: process_data"));
}

#[test]
fn test_create_release_notes_system_prompt() {
    let config = create_mock_config();
    let prompt = create_release_notes_system_prompt(&config);

    assert!(prompt.contains("You are an AI assistant specialized in generating comprehensive and user-friendly release notes"));
    assert!(prompt.contains("include tasteful, appropriate, and intelligent use of emojis"));
    assert!(prompt.contains("Always mention performance impacts"));
    assert!(prompt.contains("Incorporate the overall metrics"));
}

#[test]
fn test_create_release_notes_user_prompt() {
    let changelog =
        "## Features\n- Added new processing capability\n## Bug Fixes\n- Fixed memory leak";

    // Test Minimal detail level
    let minimal_prompt =
        create_release_notes_user_prompt(changelog, DetailLevel::Minimal, "v1.0.0", "v1.1.0");
    assert!(minimal_prompt
        .contains("Based on the following changelog for changes from v1.0.0 to v1.1.0"));
    assert!(minimal_prompt.contains("generate concise release notes"));
    assert!(minimal_prompt.contains("Keep the release notes brief"));

    // Test Standard detail level
    let standard_prompt =
        create_release_notes_user_prompt(changelog, DetailLevel::Standard, "v1.0.0", "v1.1.0");
    assert!(standard_prompt.contains("generate comprehensive release notes"));
    assert!(standard_prompt.contains("Provide a balanced overview"));

    // Test Detailed detail level
    let detailed_prompt =
        create_release_notes_user_prompt(changelog, DetailLevel::Detailed, "v1.0.0", "v1.1.0");
    assert!(detailed_prompt.contains("generate highly detailed release notes"));
    assert!(detailed_prompt.contains("Include detailed explanations"));
}
