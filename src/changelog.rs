use crate::changelog_prompts;
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

        let system_prompt = changelog_prompts::create_changelog_system_prompt(config);
        let user_prompt = changelog_prompts::create_changelog_user_prompt(
            &analyzed_changes,
            detail_level,
            from,
            to,
        );

        let provider_type = LLMProviderType::from_str(&config.default_provider)
            .context("Failed to parse default provider")?;

        let changelog =
            llm::get_refined_message(config, &provider_type, &system_prompt, &user_prompt, None)
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

        let system_prompt = changelog_prompts::create_release_notes_system_prompt(config);
        let user_prompt =
            changelog_prompts::create_release_notes_user_prompt(&changelog, detail_level, from, to);

        let provider_type = LLMProviderType::from_str(&config.default_provider)
            .context("Failed to parse default provider")?;

        let release_notes =
            llm::get_refined_message(config, &provider_type, &system_prompt, &user_prompt, None)
                .await
                .context("Failed to generate release notes summary")?;

        Ok(release_notes)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DetailLevel {
    Minimal,
    Standard,
    Detailed,
}

impl DetailLevel {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(DetailLevel::Minimal),
            "standard" => Ok(DetailLevel::Standard),
            "detailed" => Ok(DetailLevel::Detailed),
            _ => Err(anyhow::anyhow!("Invalid detail level: {}", s)),
        }
    }
}
