use super::models::ChangeMetrics;
use super::readme_reader::get_readme_summary;
use crate::changes::change_analyzer::{AnalyzedChange, ChangeAnalyzer};
use crate::common::DetailLevel;
use crate::config::Config;
use crate::git::GitRepo;
use crate::llm;
use crate::llm_providers::LLMProviderType;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::Arc;

type UserPromptFn =
    fn(&[AnalyzedChange], &ChangeMetrics, DetailLevel, &str, &str, Option<&str>) -> String;

pub async fn generate_changes_content<T>(
    git_repo: Arc<GitRepo>,
    from: &str,
    to: &str,
    config: &Config,
    detail_level: DetailLevel,
    create_system_prompt: fn(&Config) -> String,
    create_user_prompt: UserPromptFn,
) -> Result<T>
where
    T: DeserializeOwned + Serialize + Debug,
    String: Into<T>,
{
    // Create ChangeAnalyzer with Arc<GitRepo>
    let analyzer = ChangeAnalyzer::new(git_repo.clone())?;

    // Get analyzed changes
    let analyzed_changes = analyzer.analyze_commits(from, to)?;

    // Get metrics
    let total_metrics = analyzer.calculate_total_metrics(&analyzed_changes);

    // Get README summary for context
    let provider_type: LLMProviderType = config.default_provider.parse()?;
    let readme_summary = get_readme_summary(git_repo, to, config, &provider_type)
        .await
        .context("Failed to get README summary")?;

    // Create prompts for the LLM
    let system_prompt = create_system_prompt(config);
    let user_prompt = create_user_prompt(
        &analyzed_changes,
        &total_metrics,
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
