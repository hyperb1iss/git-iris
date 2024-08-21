use crate::changes::change_analyzer::{AnalyzedChange, ChangeAnalyzer};
use crate::changes::readme_reader;
use crate::common::DetailLevel;
use crate::config::Config;
use crate::llm;
use crate::llm_providers::LLMProviderType;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::path::Path;

pub async fn generate_changes_content<T>(
    repo_path: &Path,
    from: &str,
    to: &str,
    config: &Config,
    detail_level: DetailLevel,
    create_system_prompt: fn(&Config) -> String,
    create_user_prompt: fn(&[AnalyzedChange], DetailLevel, &str, &str, Option<&str>) -> String,
) -> Result<T>
where
    T: DeserializeOwned + Serialize + Debug,
    String: Into<T>,
{
    // Get analyzed changes
    let analyzed_changes = ChangeAnalyzer::analyze_commits(repo_path, from, to)?;

    // Get README summary for context
    let provider_type: LLMProviderType = config.default_provider.parse()?;
    let readme_summary = readme_reader::get_readme_summary(repo_path, to, config, &provider_type)
        .await
        .context("Failed to get README summary")?;

    // Create prompts for the LLM
    let system_prompt = create_system_prompt(config);
    let user_prompt = create_user_prompt(
        &analyzed_changes,
        detail_level,
        from,
        to,
        readme_summary.as_deref(),
    );

    // Generate content using LLM
    llm::get_refined_message::<T>(config, &provider_type, &system_prompt, &user_prompt)
        .await
        .context("Failed to generate content")
}
