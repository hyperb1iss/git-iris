use anyhow::Result;
use git2::Repository;
use git_iris::changes::models::{
    ChangeEntry, ChangeMetrics, ChangelogResponse, ChangelogType, ReleaseNotesResponse,
};
use git_iris::common::DetailLevel;

use std::path::Path;
use std::str::FromStr;
use tempfile::TempDir;

/// Sets up a temporary Git repository for testing
#[allow(dead_code)]
fn setup_test_repo() -> Result<(TempDir, Repository)> {
    let temp_dir = TempDir::new()?;
    let repo = Repository::init(temp_dir.path())?;

    let signature = git2::Signature::now("Test User", "test@example.com")?;

    // Create initial commit
    {
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;
    }

    // Create a tag for the initial commit (v1.0.0)
    {
        let head = repo.head()?.peel_to_commit()?;
        repo.tag(
            "v1.0.0",
            &head.into_object(),
            &signature,
            "Version 1.0.0",
            false,
        )?;
    }

    // Create a new file and commit
    std::fs::write(temp_dir.path().join("file1.txt"), "Hello, world!")?;
    {
        let mut index = repo.index()?;
        index.add_path(Path::new("file1.txt"))?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let parent = repo.head()?.peel_to_commit()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Add file1.txt",
            &tree,
            &[&parent],
        )?;
    }

    // Create another tag (v1.1.0)
    {
        let head = repo.head()?.peel_to_commit()?;
        repo.tag(
            "v1.1.0",
            &head.into_object(),
            &signature,
            "Version 1.1.0",
            false,
        )?;
    }

    Ok((temp_dir, repo))
}

#[test]
fn test_changelog_response_structure() {
    let changelog_response = ChangelogResponse {
        version: Some("1.0.0".to_string()),
        release_date: Some("2023-06-01".to_string()),
        sections: {
            let mut sections = std::collections::HashMap::new();
            sections.insert(
                ChangelogType::Added,
                vec![ChangeEntry {
                    description: "New feature added".to_string(),
                    commit_hashes: vec!["abc123".to_string()],
                    associated_issues: vec!["#123".to_string()],
                    pull_request: Some("PR #456".to_string()),
                }],
            );
            sections
        },
        breaking_changes: vec![],
        metrics: ChangeMetrics {
            total_commits: 1,
            files_changed: 1,
            insertions: 10,
            deletions: 5,
            total_lines_changed: 15,
        },
    };

    assert!(changelog_response.version.is_some());
    assert!(changelog_response.release_date.is_some());
    assert!(!changelog_response.sections.is_empty());
    assert!(changelog_response
        .sections
        .contains_key(&ChangelogType::Added));
    assert!(changelog_response.metrics.total_commits > 0);
    assert!(changelog_response.metrics.files_changed > 0);
}

#[test]
fn test_release_notes_response_structure() {
    let release_notes_response = ReleaseNotesResponse {
        version: Some("1.0.0".to_string()),
        release_date: Some("2023-06-01".to_string()),
        summary: "This release includes new features and bug fixes.".to_string(),
        highlights: vec![],
        sections: vec![],
        breaking_changes: vec![],
        upgrade_notes: vec![],
        metrics: ChangeMetrics {
            total_commits: 1,
            files_changed: 1,
            insertions: 10,
            deletions: 5,
            total_lines_changed: 1000,
        },
    };

    assert!(release_notes_response.version.is_some());
    assert!(release_notes_response.release_date.is_some());
    assert!(!release_notes_response.summary.is_empty());
    assert!(release_notes_response.metrics.total_commits > 0);
    assert!(release_notes_response.metrics.files_changed > 0);
}

#[test]
fn test_detail_level_from_str() {
    assert_eq!(
        DetailLevel::from_str("minimal").expect("Failed to parse 'minimal'"),
        DetailLevel::Minimal,
        "Should parse 'minimal' correctly"
    );
    assert_eq!(
        DetailLevel::from_str("standard").expect("Failed to parse 'standard'"),
        DetailLevel::Standard,
        "Should parse 'standard' correctly"
    );
    assert_eq!(
        DetailLevel::from_str("detailed").expect("Failed to parse 'detailed'"),
        DetailLevel::Detailed,
        "Should parse 'detailed' correctly"
    );
    assert!(
        DetailLevel::from_str("invalid").is_err(),
        "Should return an error for invalid input"
    );
}
