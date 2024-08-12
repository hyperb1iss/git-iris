use super::prompt;
use super::readme_reader::{find_and_read_readme, summarize_readme};
use crate::common::DetailLevel;
use crate::config::Config;
use crate::git;
use crate::llm;
use crate::llm_providers::LLMProviderType;
use anyhow::{Context, Result};
use std::path::Path;

pub struct ChangelogGenerator;

impl ChangelogGenerator {
    pub async fn generate(
        repo_path: &Path,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let analyzed_changes = git::get_commits_between(repo_path, from, to)?;

        // Find and summarize README
        let readme_content = find_and_read_readme(repo_path)?;
        let readme_summary = if let Some(content) = readme_content {
            let provider_type: LLMProviderType = config.default_provider.parse()?;
            Some(summarize_readme(config, &provider_type, &content).await?)
        } else {
            None
        };

        let mut system_prompt = prompt::create_changelog_system_prompt(config);
        let effective_instructions = config.get_effective_instructions();
        if !effective_instructions.is_empty() {
            system_prompt.push_str(&format!(
                "\n\nAdditional instructions:\n{}",
                effective_instructions
            ));
        }

        let user_prompt = prompt::create_changelog_user_prompt(
            &analyzed_changes,
            detail_level,
            from,
            to,
            readme_summary.as_deref(),
        );

        let provider_type: LLMProviderType = config
            .default_provider
            .parse()
            .context("Failed to parse default provider")?;

        let changelog =
            llm::get_refined_message(config, &provider_type, &system_prompt, &user_prompt)
                .await
                .context("Failed to generate changelog")?;

        Ok(changelog)
    }
}

pub struct ReleaseNotesGenerator;

impl ReleaseNotesGenerator {
    pub async fn generate(
        repo_path: &Path,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let changelog =
            ChangelogGenerator::generate(repo_path, from, to, config, detail_level).await?;

        // Find and summarize README
        let readme_content = find_and_read_readme(repo_path)?;
        let readme_summary = if let Some(content) = readme_content {
            let provider_type: LLMProviderType = config.default_provider.parse()?;
            Some(summarize_readme(config, &provider_type, &content).await?)
        } else {
            None
        };

        let mut system_prompt = prompt::create_release_notes_system_prompt(config);
        let effective_instructions = config.get_effective_instructions();
        if !effective_instructions.is_empty() {
            system_prompt.push_str(&format!(
                "\n\nAdditional instructions:\n{}",
                effective_instructions
            ));
        }

        let user_prompt = prompt::create_release_notes_user_prompt(
            &changelog,
            detail_level,
            from,
            to,
            readme_summary.as_deref(),
        );

        let provider_type: LLMProviderType = config
            .default_provider
            .parse()
            .context("Failed to parse default provider")?;

        let release_notes =
            llm::get_refined_message(config, &provider_type, &system_prompt, &user_prompt)
                .await
                .context("Failed to generate release notes summary")?;

        Ok(release_notes)
    }
}
