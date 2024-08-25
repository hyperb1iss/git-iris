use super::common::generate_changes_content;
use super::models::{BreakingChange, ChangeEntry, ChangeMetrics, ChangelogResponse, ChangelogType};
use super::prompt;
use crate::common::DetailLevel;
use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

/// Struct responsible for generating changelogs
pub struct ChangelogGenerator;

impl ChangelogGenerator {
    /// Generates a changelog for the specified range of commits.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the Git repository
    /// * `from` - Starting point for the changelog (e.g., a commit hash or tag)
    /// * `to` - Ending point for the changelog (e.g., a commit hash, tag, or "HEAD")
    /// * `config` - Configuration object containing LLM settings
    /// * `detail_level` - Level of detail for the changelog (Minimal, Standard, or Detailed)
    ///
    /// # Returns
    ///
    /// A Result containing the generated changelog as a String, or an error
    pub async fn generate(
        repo_path: &Path,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let changelog: ChangelogResponse = generate_changes_content::<ChangelogResponse>(
            repo_path,
            from,
            to,
            config,
            detail_level,
            prompt::create_changelog_system_prompt,
            prompt::create_changelog_user_prompt,
        )
        .await?;

        Ok(format_changelog_response(&changelog))
    }
}

/// Formats the `ChangelogResponse` into a human-readable changelog
fn format_changelog_response(response: &ChangelogResponse) -> String {
    let mut formatted = String::new();

    // Add header
    formatted.push_str(&"# Changelog\n\n".bright_cyan().bold().to_string());
    formatted.push_str("All notable changes to this project will be documented in this file.\n\n");
    formatted.push_str(
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n",
    );
    formatted.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

    // Add version and release date
    formatted.push_str(&format!(
        "## [{}] - {}\n\n",
        response
            .version
            .clone()
            .unwrap_or_default()
            .bright_green()
            .bold(),
        response.release_date.clone().unwrap_or_default().yellow()
    ));

    // Add changes grouped by type
    for (change_type, entries) in &response.sections {
        if !entries.is_empty() {
            formatted.push_str(&format_change_type(change_type));
            for entry in entries {
                formatted.push_str(&format_change_entry(entry));
            }
            formatted.push('\n');
        }
    }

    // Add breaking changes
    if !response.breaking_changes.is_empty() {
        formatted.push_str(&"### ⚠️ Breaking Changes\n\n".bright_red().bold().to_string());
        for breaking_change in &response.breaking_changes {
            formatted.push_str(&format_breaking_change(breaking_change));
        }
        formatted.push('\n');
    }

    // Add metrics
    formatted.push_str(&"### 📊 Metrics\n\n".bright_magenta().bold().to_string());
    formatted.push_str(&format_metrics(&response.metrics));

    formatted
}

/// Formats a change type with an appropriate emoji
fn format_change_type(change_type: &ChangelogType) -> String {
    let (emoji, text) = match change_type {
        ChangelogType::Added => ("✨", "Added"),
        ChangelogType::Changed => ("🔄", "Changed"),
        ChangelogType::Deprecated => ("⚠️", "Deprecated"),
        ChangelogType::Removed => ("🗑️", "Removed"),
        ChangelogType::Fixed => ("🐛", "Fixed"),
        ChangelogType::Security => ("🔒", "Security"),
    };
    format!("### {} {}\n\n", emoji, text.bright_blue().bold())
}

/// Formats a single change entry
fn format_change_entry(entry: &ChangeEntry) -> String {
    let mut formatted = format!("- {}", entry.description);

    if !entry.associated_issues.is_empty() {
        formatted.push_str(&format!(
            " ({})",
            entry.associated_issues.join(", ").yellow()
        ));
    }

    if let Some(pr) = &entry.pull_request {
        formatted.push_str(&format!(" [{}]", pr.bright_purple()));
    }

    formatted.push_str(&format!(" ({})\n", entry.commit_hashes.join(", ").dimmed()));

    formatted
}

/// Formats a breaking change
fn format_breaking_change(breaking_change: &BreakingChange) -> String {
    format!(
        "- {} ({})\n",
        breaking_change.description,
        breaking_change.commit_hash.dimmed()
    )
}

/// Formats the change metrics
fn format_metrics(metrics: &ChangeMetrics) -> String {
    format!(
        "- Total Commits: {}\n- Files Changed: {}\n- Insertions: {}\n- Deletions: {}\n",
        metrics.total_commits.to_string().green(),
        metrics.files_changed.to_string().yellow(),
        metrics.insertions.to_string().green(),
        metrics.deletions.to_string().red()
    )
}
