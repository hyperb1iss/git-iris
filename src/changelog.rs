use crate::changelog_prompts;
use crate::config::Config;
use crate::git;
use crate::llm_providers::{create_provider, LLMProviderType};
use anyhow::{Context, Result};
use std::path::Path;

pub struct ChangelogGenerator;

impl ChangelogGenerator {
    pub async fn generate(
        repo_path: &Path,
        from: &str,
        to: &str,
        config: &Config,
    ) -> Result<String> {
        let commits = git::get_commits_between(repo_path, from, to)?;
        let provider_type = LLMProviderType::from_str(&config.default_provider)?;

        let system_prompt = changelog_prompts::create_changelog_system_prompt(config);
        let user_prompt = changelog_prompts::create_changelog_user_prompt(commits);

        let provider_config = config
            .get_provider_config(&config.default_provider)
            .ok_or_else(|| anyhow::anyhow!("Provider configuration not found"))?
            .to_llm_provider_config();

        let llm_provider = create_provider(provider_type, provider_config)?;

        let changelog = llm_provider
            .generate_message(&system_prompt, &user_prompt)
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
    ) -> Result<String> {
        let changelog = ChangelogGenerator::generate(repo_path, from, to, config).await?;
        let provider_type = LLMProviderType::from_str(&config.default_provider)?;

        let system_prompt = changelog_prompts::create_release_notes_system_prompt(config);
        let user_prompt = changelog_prompts::create_release_notes_user_prompt(changelog);

        // Generate release notes summary

        let provider_config = config
            .get_provider_config(&config.default_provider)
            .ok_or_else(|| anyhow::anyhow!("Provider configuration not found"))?
            .to_llm_provider_config();

        let llm_provider = create_provider(provider_type, provider_config)?;

        let release_notes = llm_provider
            .generate_message(&system_prompt, &user_prompt)
            .await
            .context("Failed to generate release notes summary")?;

        Ok(release_notes)
    }
}
