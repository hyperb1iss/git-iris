use crate::context::{ChangeType, RecentCommit};
use crate::git::get_commits_between_with_callback;
use anyhow::{Context, Result};
use git2::{Diff, Repository};
use regex::Regex;
use std::path::Path;

use super::models::{ChangeMetrics, ChangelogType};

/// Represents the analyzed changes for a single commit
#[derive(Debug, Clone)]
pub struct AnalyzedChange {
    pub commit_hash: String,
    pub commit_message: String,
    pub author: String,
    pub file_changes: Vec<FileChange>,
    pub metrics: ChangeMetrics,
    pub impact_score: f32,
    pub change_type: ChangelogType,
    pub is_breaking_change: bool,
    pub associated_issues: Vec<String>,
    pub pull_request: Option<String>,
}

/// Represents changes to a single file
#[derive(Debug, Clone)]
pub struct FileChange {
    pub old_path: String,
    pub new_path: String,
    pub change_type: ChangeType,
    pub analysis: Vec<String>,
}

/// Analyzer for processing Git commits and generating detailed change information
pub struct ChangeAnalyzer;

impl ChangeAnalyzer {
    /// Analyze commits between two Git references
    pub fn analyze_commits(repo_path: &Path, from: &str, to: &str) -> Result<Vec<AnalyzedChange>> {
        let repo = Repository::open(repo_path).context("Failed to open repository")?;

        get_commits_between_with_callback(repo_path, from, to, |commit| {
            Self::analyze_commit(&repo, commit)
        })
    }

    /// Analyze a single commit
    fn analyze_commit(repo: &Repository, commit: &RecentCommit) -> Result<AnalyzedChange> {
        let commit_obj = repo.find_commit(commit.hash.parse()?)?;
        let parent = commit_obj.parent(0).ok();
        let diff = repo.diff_tree_to_tree(
            parent.as_ref().and_then(|c| c.tree().ok()).as_ref(),
            Some(&commit_obj.tree()?),
            None,
        )?;

        let file_changes = Self::analyze_file_changes(&diff)?;
        let metrics = Self::calculate_metrics(&diff)?;
        let change_type = Self::classify_change(&commit.message, &file_changes);
        let is_breaking_change = Self::detect_breaking_change(&commit.message, &file_changes);
        let associated_issues = Self::extract_associated_issues(&commit.message);
        let pull_request = Self::extract_pull_request(&commit.message);
        let impact_score =
            Self::calculate_impact_score(&metrics, &file_changes, is_breaking_change);

        Ok(AnalyzedChange {
            commit_hash: commit.hash.clone(),
            commit_message: commit.message.clone(),
            author: commit.author.clone(),
            file_changes,
            metrics,
            impact_score,
            change_type,
            is_breaking_change,
            associated_issues,
            pull_request,
        })
    }

    /// Analyze changes for each file in the commit
    fn analyze_file_changes(diff: &Diff) -> Result<Vec<FileChange>> {
        let mut file_changes = Vec::new();

        diff.foreach(
            &mut |delta, _| {
                let old_file = delta.old_file();
                let new_file = delta.new_file();
                let change_type = match delta.status() {
                    git2::Delta::Added => ChangeType::Added,
                    git2::Delta::Deleted => ChangeType::Deleted,
                    _ => ChangeType::Modified,
                };

                let file_change = FileChange {
                    old_path: old_file
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    new_path: new_file
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    change_type,
                    analysis: vec![], // TODO: Implement file-specific analysis if needed
                };

                file_changes.push(file_change);
                true
            },
            None,
            None,
            None,
        )?;

        Ok(file_changes)
    }

    /// Calculate metrics for the commit
    fn calculate_metrics(diff: &Diff) -> Result<ChangeMetrics> {
        let stats = diff.stats()?;
        Ok(ChangeMetrics {
            total_commits: 1,
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            total_lines_changed: stats.insertions() + stats.deletions(),
        })
    }

    /// Classify the type of change based on commit message and file changes
    fn classify_change(commit_message: &str, file_changes: &[FileChange]) -> ChangelogType {
        let message_lower = commit_message.to_lowercase();

        // First, check the commit message
        if message_lower.contains("add") || message_lower.contains("new") {
            return ChangelogType::Added;
        } else if message_lower.contains("deprecat") {
            return ChangelogType::Deprecated;
        } else if message_lower.contains("remov") || message_lower.contains("delet") {
            return ChangelogType::Removed;
        } else if message_lower.contains("fix") || message_lower.contains("bug") {
            return ChangelogType::Fixed;
        } else if message_lower.contains("secur") || message_lower.contains("vulnerab") {
            return ChangelogType::Security;
        }

        // If the commit message doesn't give us a clear indication, check the file changes
        let has_additions = file_changes
            .iter()
            .any(|fc| fc.change_type == ChangeType::Added);
        let has_deletions = file_changes
            .iter()
            .any(|fc| fc.change_type == ChangeType::Deleted);

        if has_additions && !has_deletions {
            ChangelogType::Added
        } else if has_deletions && !has_additions {
            ChangelogType::Removed
        } else {
            ChangelogType::Changed
        }
    }

    /// Detect if the change is a breaking change
    fn detect_breaking_change(commit_message: &str, file_changes: &[FileChange]) -> bool {
        let message_lower = commit_message.to_lowercase();
        if message_lower.contains("breaking change")
            || message_lower.contains("breaking-change")
            || message_lower.contains("major version")
        {
            return true;
        }

        // Check file changes for potential breaking changes
        file_changes.iter().any(|fc| {
            fc.analysis.iter().any(|analysis| {
                analysis.to_lowercase().contains("breaking change")
                    || analysis.to_lowercase().contains("api change")
                    || analysis.to_lowercase().contains("incompatible")
            })
        })
    }

    /// Extract associated issue numbers from the commit message
    fn extract_associated_issues(commit_message: &str) -> Vec<String> {
        let re = Regex::new(r"#(\d+)").expect("Could not compile regex");
        re.captures_iter(commit_message)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    /// Extract pull request number from the commit message
    fn extract_pull_request(commit_message: &str) -> Option<String> {
        let re = Regex::new(r"(?i)pull request #?(\d+)").expect("Could not compile regex");
        re.captures(commit_message).map(|cap| cap[1].to_string())
    }

    /// Calculate the impact score of the change
    fn calculate_impact_score(
        metrics: &ChangeMetrics,
        file_changes: &[FileChange],
        is_breaking_change: bool,
    ) -> f32 {
        let base_score = (metrics.total_lines_changed as f32) / 100.0;
        let file_score = file_changes.len() as f32 / 10.0;
        let breaking_change_score = if is_breaking_change { 5.0 } else { 0.0 };

        base_score + file_score + breaking_change_score
    }
}

/// Calculate total metrics for a set of analyzed changes
pub fn calculate_total_metrics(changes: &[AnalyzedChange]) -> ChangeMetrics {
    changes.iter().fold(
        ChangeMetrics {
            total_commits: changes.len(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            total_lines_changed: 0,
        },
        |mut acc, change| {
            acc.files_changed += change.metrics.files_changed;
            acc.insertions += change.metrics.insertions;
            acc.deletions += change.metrics.deletions;
            acc.total_lines_changed += change.metrics.total_lines_changed;
            acc
        },
    )
}
