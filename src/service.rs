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
        let combined_instructions =
            prompt::get_combined_instructions(&self.config, Some(instructions), Some(preset));
        let system_prompt = prompt::create_system_prompt(self.use_gitmoji, &combined_instructions);
        let user_prompt = prompt::create_user_prompt(&git_info)?;

        let message_str = llm::get_refined_message(
            &self.config,
            &self.provider_type,
            &system_prompt,
            &user_prompt,
            Some(&combined_instructions),
        )
        .await?;

        serde_json::from_str(&message_str).map_err(Error::from)
    }

    pub fn perform_commit(&self, message: &str) -> Result<()> {
        git::commit(&self.repo_path, message)
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
