//! Agent setup service for Git-Iris
//!
//! This service handles all the setup and initialization for the agent framework,
//! including configuration loading, client creation, and agent setup.

use anyhow::Result;
use std::sync::Arc;

use crate::agents::context::TaskContext;
use crate::agents::iris::StructuredResponse;
use crate::agents::{AgentBackend, IrisAgent, IrisAgentBuilder};
use crate::common::CommonParams;
use crate::config::Config;
use crate::git::GitRepo;
use crate::providers::Provider;

/// Service for setting up agents with proper configuration
pub struct AgentSetupService {
    config: Config,
    git_repo: Option<GitRepo>,
}

impl AgentSetupService {
    /// Create a new setup service with the given configuration
    pub fn new(config: Config) -> Self {
        Self {
            config,
            git_repo: None,
        }
    }

    /// Create setup service from common parameters (following existing patterns)
    pub fn from_common_params(
        common_params: &CommonParams,
        repository_url: Option<String>,
    ) -> Result<Self> {
        let mut config = Config::load()?;

        // Apply common parameters to config (following existing pattern)
        common_params.apply_to_config(&mut config)?;

        let mut setup_service = Self::new(config);

        // Setup git repo if needed
        if let Some(repo_url) = repository_url {
            // Handle remote repository setup (following existing pattern)
            setup_service.git_repo = Some(GitRepo::new_from_url(Some(repo_url))?);
        } else {
            // Use local repository
            setup_service.git_repo = Some(GitRepo::new(&std::env::current_dir()?)?);
        }

        Ok(setup_service)
    }

    /// Create a configured Iris agent
    pub fn create_iris_agent(&mut self) -> Result<IrisAgent> {
        let backend = AgentBackend::from_config(&self.config)?;
        // Validate environment (API keys etc) before creating agent
        self.validate_provider(&backend)?;

        let mut agent = IrisAgentBuilder::new()
            .with_provider(&backend.provider_name)
            .with_model(&backend.model)
            .build()?;

        // Pass config and fast model to agent
        agent.set_config(self.config.clone());
        agent.set_fast_model(backend.fast_model);

        Ok(agent)
    }

    /// Validate provider configuration (API keys etc)
    fn validate_provider(&self, backend: &AgentBackend) -> Result<()> {
        let provider: Provider = backend
            .provider_name
            .parse()
            .map_err(|_| anyhow::anyhow!("Unsupported provider: {}", backend.provider_name))?;

        // Check API key - from config or environment
        let has_api_key = self
            .config
            .get_provider_config(provider.name())
            .is_some_and(crate::providers::ProviderConfig::has_api_key);

        if !has_api_key && std::env::var(provider.api_key_env()).is_err() {
            return Err(anyhow::anyhow!(
                "No API key found for {}. Set {} or configure in ~/.config/git-iris/config.toml",
                provider.name(),
                provider.api_key_env()
            ));
        }

        Ok(())
    }

    /// Get the git repository instance
    pub fn git_repo(&self) -> Option<&GitRepo> {
        self.git_repo.as_ref()
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// High-level function to handle tasks with agents using a common pattern
/// This is a convenience function that sets up an agent and executes a task
pub async fn handle_with_agent<F, Fut, T>(
    common_params: CommonParams,
    repository_url: Option<String>,
    capability: &str,
    task_prompt: &str,
    handler: F,
) -> Result<T>
where
    F: FnOnce(crate::agents::iris::StructuredResponse) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Create setup service
    let mut setup_service = AgentSetupService::from_common_params(&common_params, repository_url)?;

    // Create agent
    let mut agent = setup_service.create_iris_agent()?;

    // Execute task with capability - now returns StructuredResponse
    let result = agent.execute_task(capability, task_prompt).await?;

    // Call the handler with the result
    handler(result).await
}

/// Simple factory function for creating agents with minimal configuration
pub fn create_agent_with_defaults(provider: &str, model: &str) -> Result<IrisAgent> {
    IrisAgentBuilder::new()
        .with_provider(provider)
        .with_model(model)
        .build()
}

/// Create an agent from environment variables
pub fn create_agent_from_env() -> Result<IrisAgent> {
    let provider_str = std::env::var("IRIS_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let provider: Provider = provider_str.parse().unwrap_or_default();

    let model =
        std::env::var("IRIS_MODEL").unwrap_or_else(|_| provider.default_model().to_string());

    create_agent_with_defaults(provider.name(), &model)
}

// =============================================================================
// IrisAgentService - The primary interface for agent task execution
// =============================================================================

/// High-level service for executing agent tasks with structured context.
///
/// This is the primary interface for all agent-based operations in git-iris.
/// It handles:
/// - Configuration management
/// - Agent lifecycle
/// - Task context validation and formatting
/// - Environment validation
///
/// # Example
/// ```ignore
/// let service = IrisAgentService::from_common_params(&params, None)?;
/// let context = TaskContext::for_gen();
/// let result = service.execute_task("commit", context).await?;
/// ```
pub struct IrisAgentService {
    config: Config,
    git_repo: Option<Arc<GitRepo>>,
    provider: String,
    model: String,
    fast_model: String,
}

impl IrisAgentService {
    /// Create a new service with explicit provider configuration
    pub fn new(config: Config, provider: String, model: String, fast_model: String) -> Self {
        Self {
            config,
            git_repo: None,
            provider,
            model,
            fast_model,
        }
    }

    /// Create service from common CLI parameters
    ///
    /// This is the primary constructor for CLI usage. It:
    /// - Loads and applies configuration
    /// - Sets up the git repository (local or remote)
    /// - Validates the environment
    pub fn from_common_params(
        common_params: &CommonParams,
        repository_url: Option<String>,
    ) -> Result<Self> {
        let mut config = Config::load()?;
        common_params.apply_to_config(&mut config)?;

        // Determine backend (provider/model) from config
        let backend = AgentBackend::from_config(&config)?;

        let mut service = Self::new(
            config,
            backend.provider_name,
            backend.model,
            backend.fast_model,
        );

        // Setup git repo
        if let Some(repo_url) = repository_url {
            service.git_repo = Some(Arc::new(GitRepo::new_from_url(Some(repo_url))?));
        } else {
            service.git_repo = Some(Arc::new(GitRepo::new(&std::env::current_dir()?)?));
        }

        Ok(service)
    }

    /// Check that the environment is properly configured
    pub fn check_environment(&self) -> Result<()> {
        self.config.check_environment()
    }

    /// Execute an agent task with structured context
    ///
    /// # Arguments
    /// * `capability` - The capability to invoke (e.g., "commit", "review", "pr")
    /// * `context` - Structured context describing what to analyze
    ///
    /// # Returns
    /// The structured response from the agent
    pub async fn execute_task(
        &self,
        capability: &str,
        context: TaskContext,
    ) -> Result<StructuredResponse> {
        // Create the agent
        let mut agent = self.create_agent()?;

        // Build task prompt with context information and any custom instructions from config
        let task_prompt = Self::build_task_prompt(
            capability,
            &context,
            self.config.temp_instructions.as_deref(),
        );

        // Execute the task
        agent.execute_task(capability, &task_prompt).await
    }

    /// Execute a task with a custom prompt (for backwards compatibility)
    pub async fn execute_task_with_prompt(
        &self,
        capability: &str,
        task_prompt: &str,
    ) -> Result<StructuredResponse> {
        let mut agent = self.create_agent()?;
        agent.execute_task(capability, task_prompt).await
    }

    /// Execute an agent task with style overrides
    ///
    /// Allows runtime override of preset and gitmoji settings without
    /// modifying the underlying config. Useful for UI flows where the
    /// user can change settings per-invocation.
    ///
    /// # Arguments
    /// * `capability` - The capability to invoke
    /// * `context` - Structured context describing what to analyze
    /// * `preset` - Optional preset name override (e.g., "conventional", "cosmic")
    /// * `use_gitmoji` - Optional gitmoji setting override
    /// * `instructions` - Optional custom instructions from the user
    pub async fn execute_task_with_style(
        &self,
        capability: &str,
        context: TaskContext,
        preset: Option<&str>,
        use_gitmoji: Option<bool>,
        instructions: Option<&str>,
    ) -> Result<StructuredResponse> {
        // Clone config and apply style overrides
        let mut config = self.config.clone();
        if let Some(p) = preset {
            config.temp_preset = Some(p.to_string());
        }
        if let Some(gitmoji) = use_gitmoji {
            config.use_gitmoji = gitmoji;
        }

        // Create agent with modified config
        let mut agent = IrisAgentBuilder::new()
            .with_provider(&self.provider)
            .with_model(&self.model)
            .build()?;
        agent.set_config(config);
        agent.set_fast_model(self.fast_model.clone());

        // Build task prompt with context information and optional instructions
        let task_prompt = Self::build_task_prompt(capability, &context, instructions);

        // Execute the task
        agent.execute_task(capability, &task_prompt).await
    }

    /// Build a task prompt incorporating the context information and optional instructions
    fn build_task_prompt(
        capability: &str,
        context: &TaskContext,
        instructions: Option<&str>,
    ) -> String {
        let context_json = context.to_prompt_context();
        let diff_hint = context.diff_hint();

        // Build instruction suffix if provided
        let instruction_suffix = instructions
            .filter(|i| !i.trim().is_empty())
            .map(|i| format!("\n\n## Custom Instructions\n{}", i))
            .unwrap_or_default();

        // Extract version and date info if this is a Changelog context
        let version_info = if let TaskContext::Changelog {
            version_name, date, ..
        } = context
        {
            let version_str = version_name
                .as_ref()
                .map_or_else(|| "(derive from git refs)".to_string(), String::clone);
            format!(
                "\n\n## Version Information\n- Version: {}\n- Release Date: {}\n\nIMPORTANT: Use the exact version name and date provided above. Do NOT guess or make up version numbers or dates.",
                version_str, date
            )
        } else {
            String::new()
        };

        match capability {
            "commit" => format!(
                "Generate a commit message for the following context:\n{}\n\nUse: {}{}",
                context_json, diff_hint, instruction_suffix
            ),
            "review" => format!(
                "Review the code changes for the following context:\n{}\n\nUse: {}{}",
                context_json, diff_hint, instruction_suffix
            ),
            "pr" => format!(
                "Generate a pull request description for:\n{}\n\nUse: {}{}",
                context_json, diff_hint, instruction_suffix
            ),
            "changelog" => format!(
                "Generate a changelog for:\n{}\n\nUse: {}{}{}",
                context_json, diff_hint, version_info, instruction_suffix
            ),
            "release_notes" => format!(
                "Generate release notes for:\n{}\n\nUse: {}{}{}",
                context_json, diff_hint, version_info, instruction_suffix
            ),
            _ => format!(
                "Execute task with context:\n{}\n\nHint: {}{}",
                context_json, diff_hint, instruction_suffix
            ),
        }
    }

    /// Create a configured Iris agent
    fn create_agent(&self) -> Result<IrisAgent> {
        let mut agent = IrisAgentBuilder::new()
            .with_provider(&self.provider)
            .with_model(&self.model)
            .build()?;

        // Pass config and fast model to agent
        agent.set_config(self.config.clone());
        agent.set_fast_model(self.fast_model.clone());

        Ok(agent)
    }

    /// Create a configured Iris agent with content update tools (for Studio chat)
    fn create_agent_with_content_updates(
        &self,
        sender: crate::agents::tools::ContentUpdateSender,
    ) -> Result<IrisAgent> {
        let mut agent = self.create_agent()?;
        agent.set_content_update_sender(sender);
        Ok(agent)
    }

    /// Execute a chat task with content update capabilities
    ///
    /// This is used by Studio to enable Iris to update content via tool calls.
    pub async fn execute_chat_with_updates(
        &self,
        task_prompt: &str,
        content_update_sender: crate::agents::tools::ContentUpdateSender,
    ) -> Result<StructuredResponse> {
        let mut agent = self.create_agent_with_content_updates(content_update_sender)?;
        agent.execute_task("chat", task_prompt).await
    }

    /// Execute a chat task with streaming and content update capabilities
    ///
    /// Combines streaming output with tool-based content updates for the TUI chat.
    pub async fn execute_chat_streaming<F>(
        &self,
        task_prompt: &str,
        content_update_sender: crate::agents::tools::ContentUpdateSender,
        on_chunk: F,
    ) -> Result<StructuredResponse>
    where
        F: FnMut(&str, &str) + Send,
    {
        let mut agent = self.create_agent_with_content_updates(content_update_sender)?;
        agent
            .execute_task_streaming("chat", task_prompt, on_chunk)
            .await
    }

    /// Execute an agent task with streaming
    ///
    /// This method streams LLM output in real-time, calling the callback with each
    /// text chunk as it arrives. Ideal for TUI display of generation progress.
    ///
    /// # Arguments
    /// * `capability` - The capability to invoke (e.g., "review", "pr", "changelog")
    /// * `context` - Structured context describing what to analyze
    /// * `on_chunk` - Callback receiving `(chunk, aggregated_text)` for each delta
    ///
    /// # Returns
    /// The final structured response after streaming completes
    pub async fn execute_task_streaming<F>(
        &self,
        capability: &str,
        context: TaskContext,
        on_chunk: F,
    ) -> Result<StructuredResponse>
    where
        F: FnMut(&str, &str) + Send,
    {
        let mut agent = self.create_agent()?;
        let task_prompt = Self::build_task_prompt(
            capability,
            &context,
            self.config.temp_instructions.as_deref(),
        );
        agent
            .execute_task_streaming(capability, &task_prompt, on_chunk)
            .await
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Get the git repository if available
    pub fn git_repo(&self) -> Option<&Arc<GitRepo>> {
        self.git_repo.as_ref()
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the fast model name (for subagents and simple tasks)
    pub fn fast_model(&self) -> &str {
        &self.fast_model
    }

    /// Get the API key for the current provider from config
    pub fn api_key(&self) -> Option<String> {
        self.config
            .get_provider_config(&self.provider)
            .filter(|pc| !pc.api_key.is_empty())
            .map(|pc| pc.api_key.clone())
    }
}
