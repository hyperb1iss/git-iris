use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::prompt::{create_system_prompt, create_user_prompt, process_commit_message};
use crate::config::Config;
use crate::context::{CommitContext, GeneratedMessage};
use crate::git::{CommitResult, GitRepo};
use crate::llm;
use crate::llm_providers::LLMProviderType;

/// Service for handling Git commit operations with AI assistance
pub struct IrisCommitService {
    config: Config,
    repo: Arc<GitRepo>,
    provider_type: LLMProviderType,
    use_gitmoji: bool,
    verify: bool,
    cached_context: Arc<RwLock<Option<CommitContext>>>,
}

impl IrisCommitService {
    /// Create a new `IrisCommitService` instance
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the service
    /// * `repo_path` - The path to the Git repository
    /// * `provider_type` - The type of LLM provider to use
    /// * `use_gitmoji` - Whether to use Gitmoji in commit messages
    /// * `verify` - Whether to verify commits
    ///
    /// # Returns
    ///
    /// A Result containing the new `IrisCommitService` instance or an error
    pub fn new(
        config: Config,
        repo_path: &Path,
        provider_type: LLMProviderType,
        use_gitmoji: bool,
        verify: bool,
    ) -> Result<Self> {
        Ok(Self {
            config,
            repo: Arc::new(GitRepo::new(repo_path)?),
            provider_type,
            use_gitmoji,
            verify,
            cached_context: Arc::new(RwLock::new(None)),
        })
    }

    /// Check the environment for necessary prerequisites
    pub fn check_environment(&self) -> Result<()> {
        self.config.check_environment()
    }

    /// Get Git information for the current repository
    pub async fn get_git_info(&self) -> Result<CommitContext> {
        {
            let cached_context = self.cached_context.read().await;
            if let Some(context) = &*cached_context {
                return Ok(context.clone());
            }
        }

        let context = self.repo.get_git_info(&self.config).await?;

        {
            let mut cached_context = self.cached_context.write().await;
            *cached_context = Some(context.clone());
        }
        Ok(context)
    }

    /// Generate a commit message using AI
    ///
    /// # Arguments
    ///
    /// * `preset` - The instruction preset to use
    /// * `instructions` - Custom instructions for the AI
    ///
    /// # Returns
    ///
    /// A Result containing the generated commit message or an error
    pub async fn generate_message(
        &self,
        preset: &str,
        instructions: &str,
    ) -> anyhow::Result<GeneratedMessage> {
        let mut config_clone = self.config.clone();
        config_clone.instruction_preset = preset.to_string();
        config_clone.instructions = instructions.to_string();

        let context = self.get_git_info().await?;

        let system_prompt = create_system_prompt(&config_clone)?;
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

    /// Perform a commit with the given message
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message to use
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitResult` or an error
    pub fn perform_commit(&self, message: &str) -> Result<CommitResult> {
        let processed_message = process_commit_message(message.to_string(), self.use_gitmoji);
        if self.verify {
            self.repo.commit_and_verify(&processed_message)
        } else {
            self.repo.commit(&processed_message)
        }
    }

    /// Execute the pre-commit hook if verification is enabled
    pub fn pre_commit(&self) -> Result<()> {
        if self.verify {
            self.repo.execute_hook("pre-commit")
        } else {
            Ok(())
        }
    }

    /// Create a channel for message generation
    pub fn create_message_channel(
        &self,
    ) -> (
        mpsc::Sender<Result<GeneratedMessage>>,
        mpsc::Receiver<Result<GeneratedMessage>>,
    ) {
        mpsc::channel(1)
    }
}
