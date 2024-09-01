use super::common::generate_changes_content;
use super::models::{
    BreakingChange, ChangeMetrics, Highlight, ReleaseNotesResponse, Section, SectionItem,
};
use super::prompt;
use crate::common::DetailLevel;
use crate::config::Config;
use crate::git::GitRepo;
use anyhow::Result;
use colored::Colorize;
use std::sync::Arc;

/// Struct responsible for generating release notes
pub struct ReleaseNotesGenerator;

impl ReleaseNotesGenerator {
    /// Generates release notes for the specified range of commits.
    ///
    /// # Arguments
    ///
    /// * `git_repo` - Arc<GitRepo> instance
    /// * `from` - Starting point for the release notes (e.g., a commit hash or tag)
    /// * `to` - Ending point for the release notes (e.g., a commit hash, tag, or "HEAD")
    /// * `config` - Configuration object containing LLM settings
    /// * `detail_level` - Level of detail for the release notes (Minimal, Standard, or Detailed)
    ///
    /// # Returns
    ///
    /// A Result containing the generated release notes as a String, or an error
    pub async fn generate(
        git_repo: Arc<GitRepo>,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let release_notes: ReleaseNotesResponse = generate_changes_content::<ReleaseNotesResponse>(
            git_repo,
            from,
            to,
            config,
            detail_level,
            prompt::create_release_notes_system_prompt,
            prompt::create_release_notes_user_prompt,
        )
        .await?;

        Ok(format_release_notes_response(&release_notes))
    }
}

/// Formats the `ReleaseNotesResponse` into human-readable release notes
fn format_release_notes_response(response: &ReleaseNotesResponse) -> String {
    let mut formatted = String::new();

    // Add header
    formatted.push_str(&format!(
        "# Release Notes - v{}\n\n",
        response
            .version
            .clone()
            .unwrap_or_default()
            .bright_green()
            .bold()
    ));
    formatted.push_str(&format!(
        "Release Date: {}\n\n",
        response.release_date.clone().unwrap_or_default().yellow()
    ));

    // Add summary
    formatted.push_str(&format!("{}\n\n", response.summary.bright_cyan()));

    // Add highlights
    if !response.highlights.is_empty() {
        formatted.push_str(&"## ✨ Highlights\n\n".bright_magenta().bold().to_string());
        for highlight in &response.highlights {
            formatted.push_str(&format_highlight(highlight));
        }
    }

    // Add changes grouped by section
    for section in &response.sections {
        formatted.push_str(&format_section(section));
    }

    // Add breaking changes
    if !response.breaking_changes.is_empty() {
        formatted.push_str(&"## ⚠️ Breaking Changes\n\n".bright_red().bold().to_string());
        for breaking_change in &response.breaking_changes {
            formatted.push_str(&format_breaking_change(breaking_change));
        }
    }

    // Add upgrade notes
    if !response.upgrade_notes.is_empty() {
        formatted.push_str(&"## 🔧 Upgrade Notes\n\n".yellow().bold().to_string());
        for note in &response.upgrade_notes {
            formatted.push_str(&format!("- {note}\n"));
        }
        formatted.push('\n');
    }

    // Add metrics
    formatted.push_str(&"## 📊 Metrics\n\n".bright_blue().bold().to_string());
    formatted.push_str(&format_metrics(&response.metrics));

    formatted
}

/// Formats a highlight
fn format_highlight(highlight: &Highlight) -> String {
    format!(
        "### {}\n\n{}\n\n",
        highlight.title.bright_yellow().bold(),
        highlight.description
    )
}

/// Formats a section
fn format_section(section: &Section) -> String {
    let mut formatted = format!("## {}\n\n", section.title.bright_blue().bold());
    for item in &section.items {
        formatted.push_str(&format_section_item(item));
    }
    formatted.push('\n');
    formatted
}

/// Formats a section item
fn format_section_item(item: &SectionItem) -> String {
    let mut formatted = format!("- {}", item.description);

    if !item.associated_issues.is_empty() {
        formatted.push_str(&format!(
            " ({})",
            item.associated_issues.join(", ").yellow()
        ));
    }

    if let Some(pr) = &item.pull_request {
        formatted.push_str(&format!(" [{}]", pr.bright_purple()));
    }

    formatted.push('\n');
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
