use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::prompt::{create_system_prompt, create_user_prompt, process_commit_message};
use crate::config::Config;
use crate::context::{CommitContext, GeneratedMessage};
use crate::git;
use crate::llm;
use crate::llm_providers::LLMProviderType;

pub struct IrisCommitService {
    config: Config,
    repo_path: PathBuf,
    provider_type: LLMProviderType,
    use_gitmoji: bool,
    cached_context: Arc<RwLock<Option<CommitContext>>>,
}

impl IrisCommitService {
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
            cached_context: Arc::new(RwLock::new(None)),
        }
    }

    pub fn check_environment(&self) -> Result<()> {
        Config::check_environment()
    }

    pub async fn get_git_info(&self) -> Result<CommitContext> {
        {
            let cached_context = self.cached_context.read().await; // Await the read lock
            if let Some(context) = &*cached_context {
                return Ok(context.clone());
            }
        }

        let context = git::get_git_info(&self.repo_path, &self.config).await?;
        {
            let mut cached_context = self.cached_context.write().await; // Await the write lock
            *cached_context = Some(context.clone());
        }
        Ok(context)
    }

    pub async fn generate_message(
        &self,
        preset: &str,
        instructions: &str,
    ) -> Result<GeneratedMessage> {
        let mut config_clone = self.config.clone();
        config_clone.instruction_preset = preset.to_string();
        config_clone.instructions = instructions.to_string();

        let context = self.get_git_info().await?;

        let system_prompt = create_system_prompt(&config_clone);
        let user_prompt = create_user_prompt(&context);

        let mut generated_message = llm::get_refined_message::<GeneratedMessage>(
            &config_clone,
            &self.provider_type,
            &system_prompt,
            &user_prompt,
        )
        .await?;

        // Apply gitmoji setting
        if !self.use_gitmoji {
            generated_message.emoji = None;
        }

        Ok(generated_message)
    }

    pub fn perform_commit(&self, message: &str) -> Result<git::CommitResult> {
        let processed_message = process_commit_message(message.to_string(), self.use_gitmoji);
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
