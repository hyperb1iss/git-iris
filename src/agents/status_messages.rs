//! Dynamic status message generation using the fast model
//!
//! Generates witty, contextual waiting messages while users wait for
//! agent operations to complete. Uses fire-and-forget async with hard
//! timeout to ensure we never block on status messages.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::agents::provider::{self, DynAgent};

/// Context for generating status messages
#[derive(Debug, Clone)]
pub struct StatusContext {
    /// Type of task being performed
    pub task_type: String,
    /// Current branch name
    pub branch: Option<String>,
    /// Number of files being analyzed
    pub file_count: Option<usize>,
    /// Brief summary of what's happening (e.g., "analyzing commit changes")
    pub activity: String,
    /// Actual file names being changed (for richer context)
    pub files: Vec<String>,
    /// Whether this is a regeneration (we have more context available)
    pub is_regeneration: bool,
    /// Brief description of what's changing (e.g., "auth system, test fixes")
    pub change_summary: Option<String>,
    /// On regeneration: hint about current content (e.g., "commit about auth refactor")
    pub current_content_hint: Option<String>,
}

impl StatusContext {
    pub fn new(task_type: &str, activity: &str) -> Self {
        Self {
            task_type: task_type.to_string(),
            branch: None,
            file_count: None,
            activity: activity.to_string(),
            files: Vec::new(),
            is_regeneration: false,
            change_summary: None,
            current_content_hint: None,
        }
    }

    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    pub fn with_file_count(mut self, count: usize) -> Self {
        self.file_count = Some(count);
        self
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    pub fn with_regeneration(mut self, is_regen: bool) -> Self {
        self.is_regeneration = is_regen;
        self
    }

    pub fn with_change_summary(mut self, summary: impl Into<String>) -> Self {
        self.change_summary = Some(summary.into());
        self
    }

    pub fn with_content_hint(mut self, hint: impl Into<String>) -> Self {
        self.current_content_hint = Some(hint.into());
        self
    }
}

/// A generated status message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    /// The witty message to display
    pub message: String,
    /// Estimated time context (e.g., "a few seconds", "about 30 seconds")
    pub time_hint: Option<String>,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            message: "Working on it...".to_string(),
            time_hint: None,
        }
    }
}

/// Capitalize first letter of a string (sentence case)
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Generator for dynamic status messages
pub struct StatusMessageGenerator {
    provider: String,
    fast_model: String,
    /// API key for the provider (from config)
    api_key: Option<String>,
    /// Hard timeout for status message generation (ms)
    timeout_ms: u64,
}

impl StatusMessageGenerator {
    /// Create a new status message generator
    ///
    /// # Arguments
    /// * `provider` - LLM provider name (e.g., "anthropic", "openai")
    /// * `fast_model` - Model to use for quick generations
    /// * `api_key` - Optional API key (falls back to env var if not provided)
    pub fn new(
        provider: impl Into<String>,
        fast_model: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            fast_model: fast_model.into(),
            api_key,
            timeout_ms: 1500, // 1.5 seconds - fast model should respond quickly
        }
    }

    /// Set custom timeout in milliseconds
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Generate a status message synchronously with timeout
    ///
    /// Returns default message if generation fails or times out.
    pub async fn generate(&self, context: &StatusContext) -> StatusMessage {
        match timeout(
            Duration::from_millis(self.timeout_ms),
            self.generate_internal(context),
        )
        .await
        {
            Ok(Ok(msg)) => msg,
            Ok(Err(_)) | Err(_) => Self::default_message(context),
        }
    }

    /// Spawn fire-and-forget generation that sends result to channel
    ///
    /// This spawns an async task that will send the generated message
    /// to the provided channel. If generation times out or fails, nothing
    /// is sent (caller should already have a fallback displayed).
    pub fn spawn_generation(
        &self,
        context: StatusContext,
        tx: mpsc::UnboundedSender<StatusMessage>,
    ) {
        let provider = self.provider.clone();
        let fast_model = self.fast_model.clone();
        let api_key = self.api_key.clone();
        let timeout_ms = self.timeout_ms;

        tokio::spawn(async move {
            let generator = StatusMessageGenerator {
                provider,
                fast_model,
                api_key,
                timeout_ms,
            };

            if let Ok(Ok(msg)) = timeout(
                Duration::from_millis(timeout_ms),
                generator.generate_internal(&context),
            )
            .await
            {
                let _ = tx.send(msg);
            }
        });
    }

    /// Create a channel for receiving status messages
    pub fn create_channel() -> (
        mpsc::UnboundedSender<StatusMessage>,
        mpsc::UnboundedReceiver<StatusMessage>,
    ) {
        mpsc::unbounded_channel()
    }

    /// Build the agent for status message generation
    fn build_status_agent(
        provider: &str,
        fast_model: &str,
        api_key: Option<&str>,
    ) -> Result<DynAgent> {
        let preamble = "You write fun waiting messages for a Git AI named Iris. \
                        Concise, yet fun and encouraging, add vibes, be clever, not cheesy. \
                        Capitalize first letter, end with ellipsis. Under 35 chars. No emojis. \
                        Just the message text, nothing else.";

        match provider {
            "openai" => {
                let agent = provider::openai_builder(fast_model, api_key)
                    .preamble(preamble)
                    .max_tokens(50)
                    .build();
                Ok(DynAgent::OpenAI(agent))
            }
            "anthropic" => {
                let agent = provider::anthropic_builder(fast_model, api_key)
                    .preamble(preamble)
                    .max_tokens(50)
                    .build();
                Ok(DynAgent::Anthropic(agent))
            }
            "google" | "gemini" => {
                let agent = provider::gemini_builder(fast_model, api_key)
                    .preamble(preamble)
                    .max_tokens(50)
                    .build();
                Ok(DynAgent::Gemini(agent))
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
        }
    }

    /// Internal generation logic
    async fn generate_internal(&self, context: &StatusContext) -> Result<StatusMessage> {
        let prompt = Self::build_prompt(context);
        tracing::info!(
            "Building status agent with provider={}, model={}",
            self.provider,
            self.fast_model
        );

        // Build agent synchronously (DynClientBuilder is not Send)
        // The returned agent IS Send, so we can await after this
        let agent = match Self::build_status_agent(
            &self.provider,
            &self.fast_model,
            self.api_key.as_deref(),
        ) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to build status agent: {}", e);
                return Err(e);
            }
        };

        tracing::info!("Prompting status agent...");
        let response = match agent.prompt(&prompt).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Status agent prompt failed: {}", e);
                return Err(anyhow::anyhow!("Prompt failed: {}", e));
            }
        };

        let message = capitalize_first(response.trim());
        tracing::info!(
            "Status agent response ({} chars): {:?}",
            message.len(),
            message
        );

        // Sanity check - if response is too long or empty, use fallback
        if message.is_empty() || message.len() > 80 {
            tracing::info!("Response invalid (empty or too long), using fallback");
            return Ok(Self::default_message(context));
        }

        Ok(StatusMessage {
            message,
            time_hint: None,
        })
    }

    /// Build the prompt for status message generation
    fn build_prompt(context: &StatusContext) -> String {
        let mut prompt = String::from("Context:\n");

        prompt.push_str(&format!("Task: {}\n", context.task_type));
        prompt.push_str(&format!("Activity: {}\n", context.activity));

        if let Some(branch) = &context.branch {
            prompt.push_str(&format!("Branch: {}\n", branch));
        }

        if !context.files.is_empty() {
            let file_list: Vec<&str> = context.files.iter().take(3).map(String::as_str).collect();
            prompt.push_str(&format!("Files: {}\n", file_list.join(", ")));
        } else if let Some(count) = context.file_count {
            prompt.push_str(&format!("File count: {}\n", count));
        }

        prompt.push_str(
            "\nYour task is to use the limited context above to generate a fun waiting message \
             shown to the user while the main task executes. Concise, yet fun and encouraging. \
             Add fun vibes depending on the context. Be clever. \
             Capitalize the first letter and end with ellipsis. Under 35 chars. No emojis.\n\n\
             Just the message:",
        );
        prompt
    }

    /// Get a default message based on context (used as fallback)
    fn default_message(context: &StatusContext) -> StatusMessage {
        let message = match context.task_type.as_str() {
            "commit" => "Crafting your commit message...",
            "review" => "Analyzing code changes...",
            "pr" => "Writing PR description...",
            "changelog" => "Generating changelog...",
            "release_notes" => "Composing release notes...",
            "chat" => "Thinking...",
            "semantic_blame" => "Tracing code origins...",
            _ => "Working on it...",
        };

        StatusMessage {
            message: message.to_string(),
            time_hint: None,
        }
    }

    /// Generate a completion message when a task finishes
    pub async fn generate_completion(&self, context: &StatusContext) -> StatusMessage {
        match timeout(
            Duration::from_millis(self.timeout_ms),
            self.generate_completion_internal(context),
        )
        .await
        {
            Ok(Ok(msg)) => msg,
            Ok(Err(_)) | Err(_) => Self::default_completion(context),
        }
    }

    async fn generate_completion_internal(&self, context: &StatusContext) -> Result<StatusMessage> {
        let prompt = Self::build_completion_prompt(context);

        let agent = Self::build_status_agent(
            &self.provider,
            &self.fast_model,
            self.api_key.as_deref(),
        )?;
        let response = agent.prompt(&prompt).await?;
        let message = capitalize_first(response.trim());

        if message.is_empty() || message.len() > 80 {
            return Ok(Self::default_completion(context));
        }

        Ok(StatusMessage {
            message,
            time_hint: None,
        })
    }

    fn build_completion_prompt(context: &StatusContext) -> String {
        let mut prompt = String::from("Task just completed:\n\n");
        prompt.push_str(&format!("Task: {}\n", context.task_type));

        if let Some(branch) = &context.branch {
            prompt.push_str(&format!("Branch: {}\n", branch));
        }

        if let Some(hint) = &context.current_content_hint {
            prompt.push_str(&format!("Content: {}\n", hint));
        }

        prompt.push_str(
            "\nGenerate a brief completion message based on the content above.\n\n\
             RULES:\n\
             - Reference the SPECIFIC topic from content above (not generic \"changes\")\n\
             - Sentence case, under 35 chars, no emojis\n\
             - Just the message, nothing else:",
        );
        prompt
    }

    fn default_completion(context: &StatusContext) -> StatusMessage {
        let message = match context.task_type.as_str() {
            "commit" => "Ready to commit.",
            "review" => "Review complete.",
            "pr" => "PR description ready.",
            "changelog" => "Changelog generated.",
            "release_notes" => "Release notes ready.",
            "chat" => "Here you go.",
            "semantic_blame" => "Origins traced.",
            _ => "Done.",
        };

        StatusMessage {
            message: message.to_string(),
            time_hint: None,
        }
    }
}

/// Batch of status messages for cycling display
#[derive(Debug, Clone, Default)]
pub struct StatusMessageBatch {
    messages: Vec<StatusMessage>,
    current_index: usize,
}

impl StatusMessageBatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the batch
    pub fn add(&mut self, message: StatusMessage) {
        self.messages.push(message);
    }

    /// Get the current message (if any)
    pub fn current(&self) -> Option<&StatusMessage> {
        self.messages.get(self.current_index)
    }

    /// Advance to the next message (cycles back to start)
    pub fn next(&mut self) {
        if !self.messages.is_empty() {
            self.current_index = (self.current_index + 1) % self.messages.len();
        }
    }

    /// Check if we have any messages
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Number of messages in batch
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.current_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_context_builder() {
        let ctx = StatusContext::new("commit", "analyzing staged changes")
            .with_branch("main")
            .with_file_count(5);

        assert_eq!(ctx.task_type, "commit");
        assert_eq!(ctx.branch, Some("main".to_string()));
        assert_eq!(ctx.file_count, Some(5));
    }

    #[test]
    fn test_default_messages() {
        let ctx = StatusContext::new("commit", "test");
        let msg = StatusMessageGenerator::default_message(&ctx);
        assert_eq!(msg.message, "Crafting your commit message...");

        let ctx = StatusContext::new("review", "test");
        let msg = StatusMessageGenerator::default_message(&ctx);
        assert_eq!(msg.message, "Analyzing code changes...");

        let ctx = StatusContext::new("unknown", "test");
        let msg = StatusMessageGenerator::default_message(&ctx);
        assert_eq!(msg.message, "Working on it...");
    }

    #[test]
    fn test_message_batch_cycling() {
        let mut batch = StatusMessageBatch::new();
        assert!(batch.is_empty());
        assert!(batch.current().is_none());

        batch.add(StatusMessage {
            message: "First".to_string(),
            time_hint: None,
        });
        batch.add(StatusMessage {
            message: "Second".to_string(),
            time_hint: None,
        });

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.current().unwrap().message, "First");

        batch.next();
        assert_eq!(batch.current().unwrap().message, "Second");

        batch.next();
        assert_eq!(batch.current().unwrap().message, "First"); // Cycles back
    }

    #[test]
    fn test_prompt_building() {
        let ctx = StatusContext::new("commit", "analyzing staged changes")
            .with_branch("feature/awesome")
            .with_file_count(3);

        let prompt = StatusMessageGenerator::build_prompt(&ctx);
        assert!(prompt.contains("commit"));
        assert!(prompt.contains("analyzing staged changes"));
        assert!(prompt.contains("feature/awesome"));
        assert!(prompt.contains("3"));
    }

    /// Debug test to evaluate status message quality
    /// Run with: cargo test debug_status_messages -- --ignored --nocapture
    #[test]
    #[ignore]
    fn debug_status_messages() {
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Get provider/model from env or use defaults
            let provider =
                std::env::var("IRIS_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
            let model = std::env::var("IRIS_MODEL")
                .unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());

            println!("\n{}", "=".repeat(60));
            println!(
                "Status Message Debug - Provider: {}, Model: {}",
                provider, model
            );
            println!("{}\n", "=".repeat(60));

            let generator = StatusMessageGenerator::new(&provider, &model, None);

            // Test scenarios
            let scenarios = vec![
                StatusContext::new("commit", "crafting commit message")
                    .with_branch("main")
                    .with_files(vec![
                        "mod.rs".to_string(),
                        "status_messages.rs".to_string(),
                        "agent_tasks.rs".to_string(),
                    ])
                    .with_file_count(3),
                StatusContext::new("commit", "crafting commit message")
                    .with_branch("feature/auth")
                    .with_files(vec!["auth.rs".to_string(), "login.rs".to_string()])
                    .with_file_count(2),
                StatusContext::new("commit", "crafting commit message")
                    .with_branch("main")
                    .with_files(vec![
                        "config.ts".to_string(),
                        "App.tsx".to_string(),
                        "hooks.ts".to_string(),
                    ])
                    .with_file_count(16)
                    .with_regeneration(true)
                    .with_content_hint("refactor: simplify auth flow"),
                StatusContext::new("review", "analyzing code changes")
                    .with_branch("pr/123")
                    .with_files(vec!["reducer.rs".to_string()])
                    .with_file_count(1),
                StatusContext::new("pr", "drafting PR description")
                    .with_branch("feature/dark-mode")
                    .with_files(vec!["theme.rs".to_string(), "colors.rs".to_string()])
                    .with_file_count(5),
            ];

            for (i, ctx) in scenarios.iter().enumerate() {
                println!("--- Scenario {} ---", i + 1);
                println!(
                    "Task: {}, Branch: {:?}, Files: {:?}",
                    ctx.task_type, ctx.branch, ctx.files
                );
                if ctx.is_regeneration {
                    println!("(Regeneration, hint: {:?})", ctx.current_content_hint);
                }
                println!();

                // Generate 5 messages for each scenario
                for j in 1..=5 {
                    let msg = generator.generate(&ctx).await;
                    println!("  {}: {}", j, msg.message);
                }
                println!();
            }
        });
    }
}
