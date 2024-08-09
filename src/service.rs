use anyhow::{Error, Result};
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::context::{CommitContext, GeneratedMessage};
use crate::git;
use crate::llm;
use crate::llm_providers::LLMProviderType;
use crate::prompt;

pub struct GitIrisService {
    config: Config,
    repo_path: PathBuf,
    provider_type: LLMProviderType,
    use_gitmoji: bool,
}

impl GitIrisService {
    pub fn new(
        config: Config,
        repo_path: PathBuf,
        provider_type: LLMProviderType,
        use_gitmoji: bool,
    ) -> Self {
        Self {
            config,
            repo_path,
            provider_type,
            use_gitmoji,
        }
    }

    pub fn check_environment(&self) -> Result<()> {
        Config::check_environment()
    }

    pub fn get_git_info(&self) -> Result<CommitContext> {
        git::get_git_info(&self.repo_path, &self.config)
    }

    pub async fn generate_message(
        &self,
        preset: &str,
        instructions: &str,
    ) -> Result<GeneratedMessage> {
        let git_info = self.get_git_info()?;

        let mut config_clone = self.config.clone();
        config_clone.instruction_preset = preset.to_string();
        config_clone.instructions = instructions.to_string();

        let prompt = prompt::create_prompt(&git_info, &config_clone)?;

        let message_str =
            llm::get_refined_message(&config_clone, &self.provider_type, &prompt, "").await?;

        let mut generated_message: GeneratedMessage =
            serde_json::from_str(&message_str).map_err(Error::from)?;

        // Apply gitmoji setting
        if !self.use_gitmoji {
            generated_message.emoji = None;
        }

        Ok(generated_message)
    }

    pub fn perform_commit(&self, message: &str) -> Result<()> {
        let processed_message =
            prompt::process_commit_message(message.to_string(), self.use_gitmoji);
        git::commit(&self.repo_path, &processed_message)
    }

    pub fn create_message_channel(
        &self,
    ) -> (
        mpsc::Sender<Result<GeneratedMessage>>,
        mpsc::Receiver<Result<GeneratedMessage>>,
    ) {
        mpsc::channel(1)
    }
}
