//! Iris Agent - The unified AI agent for Git-Iris operations
//!
//! This agent can handle any Git workflow task through capability-based prompts
//! and multi-turn execution using Rig. One agent to rule them all! ✨

use anyhow::Result;
use rig::agent::{AgentBuilder, PromptResponse};
use rig::completion::CompletionModel;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

// Embed capability TOML files at compile time so they're always available
const CAPABILITY_COMMIT: &str = include_str!("capabilities/commit.toml");
const CAPABILITY_PR: &str = include_str!("capabilities/pr.toml");
const CAPABILITY_REVIEW: &str = include_str!("capabilities/review.toml");
const CAPABILITY_CHANGELOG: &str = include_str!("capabilities/changelog.toml");
const CAPABILITY_RELEASE_NOTES: &str = include_str!("capabilities/release_notes.toml");
const CAPABILITY_CHAT: &str = include_str!("capabilities/chat.toml");
const CAPABILITY_SEMANTIC_BLAME: &str = include_str!("capabilities/semantic_blame.toml");

/// Default preamble for Iris agent
const DEFAULT_PREAMBLE: &str = "\
You are Iris, a helpful AI assistant specialized in Git operations and workflows.

You have access to Git tools, code analysis tools, and powerful sub-agent capabilities for handling large analyses.

**File Access Tools:**
- **file_read** - Read file contents directly. Use `start_line` and `num_lines` for large files.
- **file_analyzer** - Get metadata and structure analysis of files.
- **code_search** - Search for patterns across files. Use sparingly; prefer file_read for known files.

**Sub-Agent Tools:**

1. **parallel_analyze** - Run multiple analysis tasks CONCURRENTLY with independent context windows
   - Best for: Large changesets (>500 lines or >20 files), batch commit analysis
   - Each task runs in its own subagent, preventing context overflow
   - Example: parallel_analyze({ \"tasks\": [\"Analyze auth/ changes for security\", \"Review db/ for performance\", \"Check api/ for breaking changes\"] })

2. **analyze_subagent** - Delegate a single focused task to a sub-agent
   - Best for: Deep dive on specific files or focused analysis

**Best Practices:**
- Use git_diff to get changes first - it includes file content
- Use file_read to read files directly instead of multiple code_search calls
- Use parallel_analyze for large changesets to avoid context overflow";

use crate::agents::provider::{self, DynAgent};
use crate::agents::tools::{GitRepoInfo, ParallelAnalyze, Workspace};

/// Trait for streaming callback to handle real-time response processing
#[async_trait::async_trait]
pub trait StreamingCallback: Send + Sync {
    /// Called when a new chunk of text is received
    async fn on_chunk(
        &self,
        chunk: &str,
        tokens: Option<crate::agents::status::TokenMetrics>,
    ) -> Result<()>;

    /// Called when the response is complete
    async fn on_complete(
        &self,
        full_response: &str,
        final_tokens: crate::agents::status::TokenMetrics,
    ) -> Result<()>;

    /// Called when an error occurs
    async fn on_error(&self, error: &anyhow::Error) -> Result<()>;

    /// Called for status updates
    async fn on_status_update(&self, message: &str) -> Result<()>;
}

/// Unified response type that can hold any structured output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuredResponse {
    CommitMessage(crate::types::GeneratedMessage),
    PullRequest(crate::types::MarkdownPullRequest),
    Changelog(crate::types::MarkdownChangelog),
    ReleaseNotes(crate::types::MarkdownReleaseNotes),
    /// Markdown-based review (LLM-driven structure)
    MarkdownReview(crate::types::MarkdownReview),
    /// Semantic blame explanation (plain text)
    SemanticBlame(String),
    PlainText(String),
}

impl fmt::Display for StructuredResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructuredResponse::CommitMessage(msg) => {
                write!(f, "{}", crate::types::format_commit_message(msg))
            }
            StructuredResponse::PullRequest(pr) => {
                write!(f, "{}", pr.raw_content())
            }
            StructuredResponse::Changelog(cl) => {
                write!(f, "{}", cl.raw_content())
            }
            StructuredResponse::ReleaseNotes(rn) => {
                write!(f, "{}", rn.raw_content())
            }
            StructuredResponse::MarkdownReview(review) => {
                write!(f, "{}", review.format())
            }
            StructuredResponse::SemanticBlame(explanation) => {
                write!(f, "{explanation}")
            }
            StructuredResponse::PlainText(text) => {
                write!(f, "{text}")
            }
        }
    }
}

/// Extract JSON from a potentially verbose response that might contain explanations
fn extract_json_from_response(response: &str) -> Result<String> {
    use crate::agents::debug;

    debug::debug_section("JSON Extraction");

    let trimmed_response = response.trim();

    // First, try parsing the entire response as JSON (for well-behaved responses)
    if trimmed_response.starts_with('{')
        && serde_json::from_str::<serde_json::Value>(trimmed_response).is_ok()
    {
        debug::debug_context_management(
            "Response is pure JSON",
            &format!("{} characters", trimmed_response.len()),
        );
        return Ok(trimmed_response.to_string());
    }

    // Try to find JSON within markdown code blocks
    if let Some(start) = response.find("```json") {
        let content_start = start + "```json".len();
        // Find the closing ``` on its own line (to avoid matching ``` inside JSON strings)
        // First try with newline prefix to find standalone closing marker
        let json_end = if let Some(end) = response[content_start..].find("\n```") {
            // Found it with newline - the JSON ends before the newline
            end
        } else {
            // Fallback: try to find ``` at start of response section or end of string
            response[content_start..]
                .find("```")
                .unwrap_or(response.len() - content_start)
        };

        let json_content = &response[content_start..content_start + json_end];
        let trimmed = json_content.trim().to_string();

        debug::debug_context_management(
            "Found JSON in markdown code block",
            &format!("{} characters", trimmed.len()),
        );

        // Save extracted JSON for debugging
        if let Err(e) = debug::write_debug_artifact("iris_extracted.json", &trimmed) {
            debug::debug_warning(&format!("Failed to write extracted JSON: {}", e));
        }

        debug::debug_json_parse_attempt(&trimmed);
        return Ok(trimmed);
    }

    // Look for JSON objects by finding { and matching }
    let mut brace_count = 0;
    let mut json_start = None;
    let mut json_end = None;

    for (i, ch) in response.char_indices() {
        match ch {
            '{' => {
                if brace_count == 0 {
                    json_start = Some(i);
                }
                brace_count += 1;
            }
            '}' => {
                brace_count -= 1;
                if brace_count == 0 && json_start.is_some() {
                    json_end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    if let (Some(start), Some(end)) = (json_start, json_end) {
        let json_content = &response[start..end];
        debug::debug_json_parse_attempt(json_content);

        // Try to sanitize before validating (control characters in strings)
        let sanitized = sanitize_json_response(json_content);

        // Validate it's actually JSON by attempting to parse it
        let _: serde_json::Value = serde_json::from_str(&sanitized).map_err(|e| {
            debug::debug_json_parse_error(&format!(
                "Found JSON-like content but it's not valid JSON: {}",
                e
            ));
            // Include more context in the error for debugging
            let preview = if json_content.len() > 200 {
                format!("{}...", &json_content[..200])
            } else {
                json_content.to_string()
            };
            anyhow::anyhow!(
                "Found JSON-like content but it's not valid JSON: {}\nPreview: {}",
                e,
                preview
            )
        })?;

        debug::debug_context_management(
            "Found valid JSON object",
            &format!("{} characters", json_content.len()),
        );
        return Ok(sanitized.into_owned());
    }

    // If no JSON found, check if the response is raw markdown that we can wrap
    // This handles cases where the model returns markdown directly without JSON wrapper
    let trimmed = response.trim();
    if trimmed.starts_with('#') || trimmed.starts_with("##") {
        debug::debug_context_management(
            "Detected raw markdown response",
            "Wrapping in JSON structure",
        );
        // Escape the markdown content for JSON and wrap it
        let escaped_content = serde_json::to_string(trimmed)?;
        // escaped_content includes quotes, so we need to use it directly as the value
        let wrapped = format!(r#"{{"content": {}}}"#, escaped_content);
        debug::debug_json_parse_attempt(&wrapped);
        return Ok(wrapped);
    }

    // If no JSON found, return error
    debug::debug_json_parse_error("No valid JSON found in response");
    Err(anyhow::anyhow!("No valid JSON found in response"))
}

/// Some providers (Anthropic) occasionally send literal control characters like newlines
/// inside JSON strings, which violates strict JSON parsing rules. This helper sanitizes
/// those responses by escaping control characters only within string literals while
/// leaving the rest of the payload untouched.
fn sanitize_json_response(raw: &str) -> Cow<'_, str> {
    let mut needs_sanitization = false;
    let mut in_string = false;
    let mut escaped = false;

    for ch in raw.chars() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                '\n' | '\r' | '\t' => {
                    needs_sanitization = true;
                    break;
                }
                c if c.is_control() => {
                    needs_sanitization = true;
                    break;
                }
                _ => {}
            }
        } else if ch == '"' {
            in_string = true;
        }
    }

    if !needs_sanitization {
        return Cow::Borrowed(raw);
    }

    let mut sanitized = String::with_capacity(raw.len());
    in_string = false;
    escaped = false;

    for ch in raw.chars() {
        if in_string {
            if escaped {
                sanitized.push(ch);
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    sanitized.push('\\');
                    escaped = true;
                }
                '"' => {
                    sanitized.push('"');
                    in_string = false;
                }
                '\n' => sanitized.push_str("\\n"),
                '\r' => sanitized.push_str("\\r"),
                '\t' => sanitized.push_str("\\t"),
                c if c.is_control() => {
                    use std::fmt::Write as _;
                    let _ = write!(&mut sanitized, "\\u{:04X}", u32::from(c));
                }
                _ => sanitized.push(ch),
            }
        } else {
            sanitized.push(ch);
            if ch == '"' {
                in_string = true;
                escaped = false;
            }
        }
    }

    Cow::Owned(sanitized)
}

/// Parse JSON with schema validation and error recovery
///
/// This function attempts to parse JSON with the following strategy:
/// 1. Try direct parsing (fast path for well-formed responses)
/// 2. If that fails, use the output validator for recovery
/// 3. Log any warnings about recovered issues
fn parse_with_recovery<T>(json_str: &str) -> Result<T>
where
    T: JsonSchema + DeserializeOwned,
{
    use crate::agents::debug as agent_debug;
    use crate::agents::output_validator::validate_and_parse;

    let validation_result = validate_and_parse::<T>(json_str)?;

    // Log recovery warnings
    if validation_result.recovered {
        agent_debug::debug_context_management(
            "JSON recovery applied",
            &format!("{} issues fixed", validation_result.warnings.len()),
        );
        for warning in &validation_result.warnings {
            agent_debug::debug_warning(warning);
        }
    }

    validation_result
        .value
        .ok_or_else(|| anyhow::anyhow!("Failed to parse JSON even after recovery"))
}

/// The unified Iris agent that can handle any Git-Iris task
///
/// Note: This struct is Send + Sync safe - we don't store the client builder,
/// instead we create it fresh when needed. This allows the agent to be used
/// across async boundaries with `tokio::spawn`.
pub struct IrisAgent {
    provider: String,
    model: String,
    /// Fast model for subagents and simple tasks
    fast_model: Option<String>,
    /// Current capability/task being executed
    current_capability: Option<String>,
    /// Provider configuration
    provider_config: HashMap<String, String>,
    /// Custom preamble
    preamble: Option<String>,
    /// Configuration for features like gitmoji, presets, etc.
    config: Option<crate::config::Config>,
    /// Optional sender for content updates (used in Studio chat mode)
    content_update_sender: Option<crate::agents::tools::ContentUpdateSender>,
    /// Persistent workspace for notes and task tracking (shared across agent invocations)
    workspace: Workspace,
}

impl IrisAgent {
    /// Create a new Iris agent with the given provider and model
    pub fn new(provider: &str, model: &str) -> Result<Self> {
        Ok(Self {
            provider: provider.to_string(),
            model: model.to_string(),
            fast_model: None,
            current_capability: None,
            provider_config: HashMap::new(),
            preamble: None,
            config: None,
            content_update_sender: None,
            workspace: Workspace::new(),
        })
    }

    /// Set the content update sender for Studio chat mode
    ///
    /// When set, the agent will have access to tools for updating
    /// commit messages, PR descriptions, and reviews.
    pub fn set_content_update_sender(&mut self, sender: crate::agents::tools::ContentUpdateSender) {
        self.content_update_sender = Some(sender);
    }

    /// Get the effective fast model (configured or same as main model)
    fn effective_fast_model(&self) -> &str {
        self.fast_model.as_deref().unwrap_or(&self.model)
    }

    /// Get the API key for the current provider from config
    fn get_api_key(&self) -> Option<&str> {
        self.config
            .as_ref()
            .and_then(|c| c.get_provider_config(&self.provider))
            .map(|pc| pc.api_key.as_str())
            .filter(|k| !k.is_empty())
    }

    /// Build the actual agent for execution
    ///
    /// Uses provider-specific builders (rig-core 0.27+) with enum dispatch for runtime
    /// provider selection. Each provider arm builds both the subagent and main agent
    /// with proper typing.
    fn build_agent(&self) -> Result<DynAgent> {
        use crate::agents::debug_tool::DebugTool;

        let preamble = self.preamble.as_deref().unwrap_or(DEFAULT_PREAMBLE);
        let fast_model = self.effective_fast_model();
        let api_key = self.get_api_key();
        let subagent_timeout = self
            .config
            .as_ref()
            .map_or(120, |c| c.subagent_timeout_secs);

        // Macro to build and configure subagent with core tools
        macro_rules! build_subagent {
            ($builder:expr) => {{
                let builder = $builder
                    .name("analyze_subagent")
                    .description("Delegate focused analysis tasks to a sub-agent with its own context window. Use for analyzing specific files, commits, or code sections independently. The sub-agent has access to Git tools (diff, log, status) and file analysis tools.")
                    .preamble("You are a specialized analysis sub-agent for Iris. Your job is to complete focused analysis tasks and return concise, actionable summaries.

Guidelines:
- Use the available tools to gather information
- Focus only on what's asked - don't expand scope
- Return a clear, structured summary of findings
- Highlight important issues, patterns, or insights
- Keep your response focused and concise")
                    .max_tokens(4096);
                let builder = self.apply_reasoning_defaults(builder);
                crate::attach_core_tools!(builder).build()
            }};
        }

        // Macro to attach main agent tools (excluding subagent which varies by type)
        macro_rules! attach_main_tools {
            ($builder:expr) => {{
                crate::attach_core_tools!($builder)
                    .tool(DebugTool::new(GitRepoInfo))
                    .tool(DebugTool::new(self.workspace.clone()))
                    .tool(DebugTool::new(ParallelAnalyze::with_timeout(
                        &self.provider,
                        fast_model,
                        subagent_timeout,
                        api_key,
                    )?))
            }};
        }

        // Macro to optionally attach content update tools
        macro_rules! maybe_attach_update_tools {
            ($builder:expr) => {{
                if let Some(sender) = &self.content_update_sender {
                    use crate::agents::tools::{UpdateCommitTool, UpdatePRTool, UpdateReviewTool};
                    $builder
                        .tool(DebugTool::new(UpdateCommitTool::new(sender.clone())))
                        .tool(DebugTool::new(UpdatePRTool::new(sender.clone())))
                        .tool(DebugTool::new(UpdateReviewTool::new(sender.clone())))
                        .build()
                } else {
                    $builder.build()
                }
            }};
        }

        match self.provider.as_str() {
            "openai" => {
                // Build subagent
                let sub_agent = build_subagent!(provider::openai_builder(fast_model, api_key)?);

                // Build main agent
                let builder = provider::openai_builder(&self.model, api_key)?
                    .preamble(preamble)
                    .max_tokens(16384);
                let builder = self.apply_reasoning_defaults(builder);
                let builder = attach_main_tools!(builder).tool(sub_agent);
                let agent = maybe_attach_update_tools!(builder);
                Ok(DynAgent::OpenAI(agent))
            }
            "anthropic" => {
                // Build subagent
                let sub_agent = build_subagent!(provider::anthropic_builder(fast_model, api_key)?);

                // Build main agent
                let builder = provider::anthropic_builder(&self.model, api_key)?
                    .preamble(preamble)
                    .max_tokens(16384);
                let builder = self.apply_reasoning_defaults(builder);
                let builder = attach_main_tools!(builder).tool(sub_agent);
                let agent = maybe_attach_update_tools!(builder);
                Ok(DynAgent::Anthropic(agent))
            }
            "google" | "gemini" => {
                // Build subagent
                let sub_agent = build_subagent!(provider::gemini_builder(fast_model, api_key)?);

                // Build main agent
                let builder = provider::gemini_builder(&self.model, api_key)?
                    .preamble(preamble)
                    .max_tokens(16384);
                let builder = self.apply_reasoning_defaults(builder);
                let builder = attach_main_tools!(builder).tool(sub_agent);
                let agent = maybe_attach_update_tools!(builder);
                Ok(DynAgent::Gemini(agent))
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", self.provider)),
        }
    }

    fn apply_reasoning_defaults<M>(&self, builder: AgentBuilder<M>) -> AgentBuilder<M>
    where
        M: CompletionModel,
    {
        if self.provider == "openai" && Self::requires_reasoning_effort(&self.model) {
            builder.additional_params(json!({
                "reasoning": {
                    "effort": "low"
                }
            }))
        } else {
            builder
        }
    }

    fn requires_reasoning_effort(model: &str) -> bool {
        let model = model.to_lowercase();
        model.starts_with("gpt-5") || model.starts_with("gpt-4.1") || model.starts_with("o1")
    }

    /// Execute task using agent with tools and parse structured JSON response
    /// This is the core method that enables Iris to use tools and generate structured outputs
    async fn execute_with_agent<T>(&self, system_prompt: &str, user_prompt: &str) -> Result<T>
    where
        T: JsonSchema + for<'a> serde::Deserialize<'a> + serde::Serialize + Send + Sync + 'static,
    {
        use crate::agents::debug;
        use crate::agents::status::IrisPhase;
        use crate::messages::get_capability_message;
        use schemars::schema_for;

        let capability = self.current_capability().unwrap_or("commit");

        debug::debug_phase_change(&format!("AGENT EXECUTION: {}", std::any::type_name::<T>()));

        // Update status - building agent (capability-aware)
        let msg = get_capability_message(capability);
        crate::iris_status_dynamic!(IrisPhase::Planning, msg.text, 2, 4);

        // Build agent with all tools attached
        let agent = self.build_agent()?;
        debug::debug_context_management(
            "Agent built with tools",
            &format!(
                "Provider: {}, Model: {} (fast: {})",
                self.provider,
                self.model,
                self.effective_fast_model()
            ),
        );

        // Create JSON schema for the response type
        let schema = schema_for!(T);
        let schema_json = serde_json::to_string_pretty(&schema)?;
        debug::debug_context_management(
            "JSON schema created",
            &format!("Type: {}", std::any::type_name::<T>()),
        );

        // Enhanced prompt that instructs Iris to use tools and respond with JSON
        let full_prompt = format!(
            "{system_prompt}\n\n{user_prompt}\n\n\
            === CRITICAL: RESPONSE FORMAT ===\n\
            After using the available tools to gather necessary information, you MUST respond with ONLY a valid JSON object.\n\n\
            REQUIRED JSON SCHEMA:\n\
            {schema_json}\n\n\
            CRITICAL INSTRUCTIONS:\n\
            - Return ONLY the raw JSON object - nothing else\n\
            - NO explanations before the JSON\n\
            - NO explanations after the JSON\n\
            - NO markdown code blocks (just raw JSON)\n\
            - NO preamble text like 'Here is the JSON:' or 'Let me generate:'\n\
            - Start your response with {{ and end with }}\n\
            - The JSON must be complete and valid\n\n\
            Your entire response should be ONLY the JSON object."
        );

        debug::debug_llm_request(&full_prompt, Some(16384));

        // Update status - generation phase (capability-aware)
        let gen_msg = get_capability_message(capability);
        crate::iris_status_dynamic!(IrisPhase::Generation, gen_msg.text, 3, 4);

        // Prompt the agent with multi-turn support
        // Set multi_turn to allow the agent to call multiple tools (default is 0 = single-shot)
        // For complex tasks like PRs and release notes, Iris may need many tool calls to analyze all changes
        // The agent knows when to stop, so we give it plenty of room (50 rounds)
        let timer = debug::DebugTimer::start("Agent prompt execution");

        debug::debug_context_management(
            "LLM request",
            "Sending prompt to agent with multi_turn(50)",
        );
        let prompt_response: PromptResponse = agent.prompt_extended(&full_prompt, 50).await?;

        timer.finish();

        // Extract usage stats for debug output
        let usage = &prompt_response.total_usage;
        debug::debug_context_management(
            "Token usage",
            &format!(
                "input: {} | output: {} | total: {}",
                usage.input_tokens, usage.output_tokens, usage.total_tokens
            ),
        );

        let response = &prompt_response.output;
        #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
        let total_tokens_usize = usage.total_tokens as usize;
        debug::debug_llm_response(
            response,
            std::time::Duration::from_secs(0),
            Some(total_tokens_usize),
        );

        // Update status - synthesis phase
        crate::iris_status_dynamic!(
            IrisPhase::Synthesis,
            "✨ Iris is synthesizing results...",
            4,
            4
        );

        // Extract and parse JSON from the response
        let json_str = extract_json_from_response(response)?;
        let sanitized_json = sanitize_json_response(&json_str);
        let sanitized_ref = sanitized_json.as_ref();

        if matches!(sanitized_json, Cow::Borrowed(_)) {
            debug::debug_json_parse_attempt(sanitized_ref);
        } else {
            debug::debug_context_management(
                "Sanitized JSON response",
                &format!("{} → {} characters", json_str.len(), sanitized_ref.len()),
            );
            debug::debug_json_parse_attempt(sanitized_ref);
        }

        // Use the output validator for robust parsing with error recovery
        let result: T = parse_with_recovery(sanitized_ref)?;

        debug::debug_json_parse_success(std::any::type_name::<T>());

        // Update status - completed
        crate::iris_status_completed!();

        Ok(result)
    }

    /// Inject style instructions into the system prompt based on config and capability
    ///
    /// Key distinction:
    /// - Commits: preset controls format (conventional = no emojis)
    /// - Non-commits (PR, review, changelog, `release_notes`): `use_gitmoji` controls emojis
    fn inject_style_instructions(&self, system_prompt: &mut String, capability: &str) {
        let Some(config) = &self.config else {
            return;
        };

        let preset_name = config.get_effective_preset_name();
        let is_conventional = preset_name == "conventional";
        let is_default_mode = preset_name == "default" || preset_name.is_empty();

        // For commits in default mode with no explicit gitmoji override, use style detection
        let use_style_detection =
            capability == "commit" && is_default_mode && config.gitmoji_override.is_none();

        // Commit emoji: respects preset (conventional = no emoji)
        let commit_emoji = config.use_gitmoji && !is_conventional && !use_style_detection;

        // Output emoji: independent of preset, only respects use_gitmoji setting
        // CLI --gitmoji/--no-gitmoji override is already applied to config.use_gitmoji
        let output_emoji = config.gitmoji_override.unwrap_or(config.use_gitmoji);

        // Inject instruction preset if configured (skip for default mode)
        if !preset_name.is_empty() && !is_default_mode {
            let library = crate::instruction_presets::get_instruction_preset_library();
            if let Some(preset) = library.get_preset(preset_name) {
                tracing::info!("📋 Injecting '{}' preset style instructions", preset_name);
                system_prompt.push_str("\n\n=== STYLE INSTRUCTIONS ===\n");
                system_prompt.push_str(&preset.instructions);
                system_prompt.push('\n');
            } else {
                tracing::warn!("⚠️ Preset '{}' not found in library", preset_name);
            }
        }

        // Handle commit-specific styling (structured JSON output with emoji field)
        if capability == "commit" {
            if use_style_detection {
                tracing::info!("🔍 Using local commit style detection (default mode)");
            } else if commit_emoji {
                system_prompt.push_str("\n\n=== GITMOJI INSTRUCTIONS ===\n");
                system_prompt.push_str("Set the 'emoji' field to a single relevant gitmoji. ");
                system_prompt.push_str(
                    "DO NOT include the emoji in the 'message' or 'title' text - only set the 'emoji' field. ",
                );
                system_prompt.push_str("Choose the most relevant emoji from this list:\n\n");
                system_prompt.push_str(&crate::gitmoji::get_gitmoji_list());
                system_prompt.push_str("\n\nThe emoji should match the primary type of change.");
            } else if is_conventional {
                system_prompt.push_str("\n\n=== CONVENTIONAL COMMITS FORMAT ===\n");
                system_prompt.push_str("IMPORTANT: This uses Conventional Commits format. ");
                system_prompt
                    .push_str("DO NOT include any emojis in the commit message or PR title. ");
                system_prompt.push_str("The 'emoji' field should be null.");
            }
        }

        // Handle non-commit outputs: use output_emoji (independent of preset)
        if capability == "pr" || capability == "review" {
            if output_emoji {
                Self::inject_pr_review_emoji_styling(system_prompt);
            } else {
                Self::inject_no_emoji_styling(system_prompt);
            }
        }

        if capability == "release_notes" && output_emoji {
            Self::inject_release_notes_emoji_styling(system_prompt);
        } else if capability == "release_notes" {
            Self::inject_no_emoji_styling(system_prompt);
        }

        if capability == "changelog" && output_emoji {
            Self::inject_changelog_emoji_styling(system_prompt);
        } else if capability == "changelog" {
            Self::inject_no_emoji_styling(system_prompt);
        }
    }

    fn inject_pr_review_emoji_styling(prompt: &mut String) {
        prompt.push_str("\n\n=== EMOJI STYLING ===\n");
        prompt.push_str("Use emojis to make the output visually scannable and engaging:\n");
        prompt.push_str("- H1 title: ONE gitmoji at the start (✨, 🐛, ♻️, etc.)\n");
        prompt.push_str("- Section headers: Add relevant emojis (🎯 What's New, ⚙️ How It Works, 📋 Commits, ⚠️ Breaking Changes)\n");
        prompt.push_str("- Commit list entries: Include gitmoji where appropriate\n");
        prompt.push_str("- Body text: Keep clean - no scattered emojis within prose\n\n");
        prompt.push_str("Choose from this gitmoji list:\n\n");
        prompt.push_str(&crate::gitmoji::get_gitmoji_list());
    }

    fn inject_release_notes_emoji_styling(prompt: &mut String) {
        prompt.push_str("\n\n=== EMOJI STYLING ===\n");
        prompt.push_str("Use at most one emoji per highlight/section title. No emojis in bullet descriptions, upgrade notes, or metrics. ");
        prompt.push_str("Pick from the approved gitmoji list (e.g., 🌟 Highlights, 🤖 Agents, 🔧 Tooling, 🐛 Fixes, ⚡ Performance). ");
        prompt.push_str("Never sprinkle emojis within sentences or JSON keys.\n\n");
        prompt.push_str(&crate::gitmoji::get_gitmoji_list());
    }

    fn inject_changelog_emoji_styling(prompt: &mut String) {
        prompt.push_str("\n\n=== EMOJI STYLING ===\n");
        prompt.push_str("Section keys must remain plain text (Added/Changed/Deprecated/Removed/Fixed/Security). ");
        prompt.push_str(
            "You may include one emoji within a change description to reinforce meaning. ",
        );
        prompt.push_str(
            "Never add emojis to JSON keys, section names, metrics, or upgrade notes.\n\n",
        );
        prompt.push_str(&crate::gitmoji::get_gitmoji_list());
    }

    fn inject_no_emoji_styling(prompt: &mut String) {
        prompt.push_str("\n\n=== NO EMOJI STYLING ===\n");
        prompt.push_str(
            "DO NOT include any emojis anywhere in the output. Keep all content plain text.",
        );
    }

    /// Execute a task with the given capability and user prompt
    ///
    /// This now automatically uses structured output based on the capability type
    pub async fn execute_task(
        &mut self,
        capability: &str,
        user_prompt: &str,
    ) -> Result<StructuredResponse> {
        use crate::agents::status::IrisPhase;
        use crate::messages::get_capability_message;

        // Show initializing status with a capability-specific message
        let waiting_msg = get_capability_message(capability);
        crate::iris_status_dynamic!(IrisPhase::Initializing, waiting_msg.text, 1, 4);

        // Load the capability config to get both prompt and output type
        let (mut system_prompt, output_type) = self.load_capability_config(capability)?;

        // Inject style instructions (presets, gitmoji, conventional commits)
        self.inject_style_instructions(&mut system_prompt, capability);

        // Set the current capability
        self.current_capability = Some(capability.to_string());

        // Update status - analyzing with agent
        crate::iris_status_dynamic!(
            IrisPhase::Analysis,
            "🔍 Iris is analyzing your changes...",
            2,
            4
        );

        // Use agent with tools for all structured outputs
        // The agent will use tools as needed and respond with JSON
        match output_type.as_str() {
            "GeneratedMessage" => {
                let response = self
                    .execute_with_agent::<crate::types::GeneratedMessage>(
                        &system_prompt,
                        user_prompt,
                    )
                    .await?;
                Ok(StructuredResponse::CommitMessage(response))
            }
            "MarkdownPullRequest" => {
                let response = self
                    .execute_with_agent::<crate::types::MarkdownPullRequest>(
                        &system_prompt,
                        user_prompt,
                    )
                    .await?;
                Ok(StructuredResponse::PullRequest(response))
            }
            "MarkdownChangelog" => {
                let response = self
                    .execute_with_agent::<crate::types::MarkdownChangelog>(
                        &system_prompt,
                        user_prompt,
                    )
                    .await?;
                Ok(StructuredResponse::Changelog(response))
            }
            "MarkdownReleaseNotes" => {
                let response = self
                    .execute_with_agent::<crate::types::MarkdownReleaseNotes>(
                        &system_prompt,
                        user_prompt,
                    )
                    .await?;
                Ok(StructuredResponse::ReleaseNotes(response))
            }
            "MarkdownReview" => {
                let response = self
                    .execute_with_agent::<crate::types::MarkdownReview>(&system_prompt, user_prompt)
                    .await?;
                Ok(StructuredResponse::MarkdownReview(response))
            }
            "SemanticBlame" => {
                // For semantic blame, we want plain text response
                let agent = self.build_agent()?;
                let full_prompt = format!("{system_prompt}\n\n{user_prompt}");
                let response = agent.prompt_multi_turn(&full_prompt, 10).await?;
                Ok(StructuredResponse::SemanticBlame(response))
            }
            _ => {
                // Fallback to regular agent for unknown types
                let agent = self.build_agent()?;
                let full_prompt = format!("{system_prompt}\n\n{user_prompt}");
                // Use multi_turn to allow tool calls even for unknown capability types
                let response = agent.prompt_multi_turn(&full_prompt, 50).await?;
                Ok(StructuredResponse::PlainText(response))
            }
        }
    }

    /// Execute a task with streaming, calling the callback with each text chunk
    ///
    /// This enables real-time display of LLM output in the TUI.
    /// The callback receives `(chunk, aggregated_text)` for each delta.
    ///
    /// Returns the final structured response after streaming completes.
    pub async fn execute_task_streaming<F>(
        &mut self,
        capability: &str,
        user_prompt: &str,
        mut on_chunk: F,
    ) -> Result<StructuredResponse>
    where
        F: FnMut(&str, &str) + Send,
    {
        use crate::agents::status::IrisPhase;
        use crate::messages::get_capability_message;
        use futures::StreamExt;
        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::{StreamedAssistantContent, StreamingPrompt};

        // Show initializing status
        let waiting_msg = get_capability_message(capability);
        crate::iris_status_dynamic!(IrisPhase::Initializing, waiting_msg.text, 1, 4);

        // Load the capability config
        let (mut system_prompt, output_type) = self.load_capability_config(capability)?;

        // Inject style instructions
        self.inject_style_instructions(&mut system_prompt, capability);

        // Set current capability
        self.current_capability = Some(capability.to_string());

        // Update status
        crate::iris_status_dynamic!(
            IrisPhase::Analysis,
            "🔍 Iris is analyzing your changes...",
            2,
            4
        );

        // Build the full prompt (simplified for streaming - no JSON schema enforcement)
        let full_prompt = format!(
            "{}\n\n{}\n\n\
            After using the available tools, respond with your analysis in markdown format.\n\
            Keep it clear, well-structured, and informative.",
            system_prompt, user_prompt
        );

        // Update status
        let gen_msg = get_capability_message(capability);
        crate::iris_status_dynamic!(IrisPhase::Generation, gen_msg.text, 3, 4);

        // Macro to consume a stream and aggregate text
        macro_rules! consume_stream {
            ($stream:expr) => {{
                let mut aggregated_text = String::new();
                let mut stream = $stream;
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text),
                        )) => {
                            aggregated_text.push_str(&text.text);
                            on_chunk(&text.text, &aggregated_text);
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall(tool_call),
                        )) => {
                            let tool_name = &tool_call.function.name;
                            let reason = format!("Calling {}", tool_name);
                            crate::iris_status_dynamic!(
                                IrisPhase::ToolExecution {
                                    tool_name: tool_name.clone(),
                                    reason: reason.clone()
                                },
                                format!("🔧 {}", reason),
                                3,
                                4
                            );
                        }
                        Ok(MultiTurnStreamItem::FinalResponse(_)) => break,
                        Err(e) => return Err(anyhow::anyhow!("Streaming error: {}", e)),
                        _ => {}
                    }
                }
                aggregated_text
            }};
        }

        // Build and stream per-provider (streaming types are model-specific)
        let aggregated_text = match self.provider.as_str() {
            "openai" => {
                let agent = self.build_openai_agent_for_streaming(&full_prompt)?;
                let stream = agent.stream_prompt(&full_prompt).multi_turn(50).await;
                consume_stream!(stream)
            }
            "anthropic" => {
                let agent = self.build_anthropic_agent_for_streaming(&full_prompt)?;
                let stream = agent.stream_prompt(&full_prompt).multi_turn(50).await;
                consume_stream!(stream)
            }
            "google" | "gemini" => {
                let agent = self.build_gemini_agent_for_streaming(&full_prompt)?;
                let stream = agent.stream_prompt(&full_prompt).multi_turn(50).await;
                consume_stream!(stream)
            }
            _ => return Err(anyhow::anyhow!("Unsupported provider: {}", self.provider)),
        };

        // Update status
        crate::iris_status_dynamic!(
            IrisPhase::Synthesis,
            "✨ Iris is synthesizing results...",
            4,
            4
        );

        let response = Self::text_to_structured_response(&output_type, aggregated_text);
        crate::iris_status_completed!();
        Ok(response)
    }

    /// Convert raw text to the appropriate structured response type
    fn text_to_structured_response(output_type: &str, text: String) -> StructuredResponse {
        match output_type {
            "MarkdownReview" => {
                StructuredResponse::MarkdownReview(crate::types::MarkdownReview { content: text })
            }
            "MarkdownPullRequest" => {
                StructuredResponse::PullRequest(crate::types::MarkdownPullRequest { content: text })
            }
            "MarkdownChangelog" => {
                StructuredResponse::Changelog(crate::types::MarkdownChangelog { content: text })
            }
            "MarkdownReleaseNotes" => {
                StructuredResponse::ReleaseNotes(crate::types::MarkdownReleaseNotes {
                    content: text,
                })
            }
            "SemanticBlame" => StructuredResponse::SemanticBlame(text),
            _ => StructuredResponse::PlainText(text),
        }
    }

    /// Build `OpenAI` agent for streaming (with tools attached)
    fn build_openai_agent_for_streaming(
        &self,
        _prompt: &str,
    ) -> Result<rig::agent::Agent<provider::OpenAIModel>> {
        use crate::agents::debug_tool::DebugTool;

        let fast_model = self.effective_fast_model();
        let api_key = self.get_api_key();
        let subagent_timeout = self
            .config
            .as_ref()
            .map_or(120, |c| c.subagent_timeout_secs);

        // Build subagent
        let sub_agent = crate::attach_core_tools!(
            provider::openai_builder(fast_model, api_key)?
                .name("analyze_subagent")
                .preamble("You are a specialized analysis sub-agent.")
                .max_tokens(4096)
        )
        .build();

        // Build main agent with tools
        let builder = provider::openai_builder(&self.model, api_key)?
            .preamble(self.preamble.as_deref().unwrap_or("You are Iris."))
            .max_tokens(16384);

        let builder = crate::attach_core_tools!(builder)
            .tool(DebugTool::new(GitRepoInfo))
            .tool(DebugTool::new(self.workspace.clone()))
            .tool(DebugTool::new(ParallelAnalyze::with_timeout(
                &self.provider,
                fast_model,
                subagent_timeout,
                api_key,
            )?))
            .tool(sub_agent);

        // Conditionally attach content update tools for chat mode
        if let Some(sender) = &self.content_update_sender {
            use crate::agents::tools::{UpdateCommitTool, UpdatePRTool, UpdateReviewTool};
            Ok(builder
                .tool(DebugTool::new(UpdateCommitTool::new(sender.clone())))
                .tool(DebugTool::new(UpdatePRTool::new(sender.clone())))
                .tool(DebugTool::new(UpdateReviewTool::new(sender.clone())))
                .build())
        } else {
            Ok(builder.build())
        }
    }

    /// Build Anthropic agent for streaming (with tools attached)
    fn build_anthropic_agent_for_streaming(
        &self,
        _prompt: &str,
    ) -> Result<rig::agent::Agent<provider::AnthropicModel>> {
        use crate::agents::debug_tool::DebugTool;

        let fast_model = self.effective_fast_model();
        let api_key = self.get_api_key();
        let subagent_timeout = self
            .config
            .as_ref()
            .map_or(120, |c| c.subagent_timeout_secs);

        // Build subagent
        let sub_agent = crate::attach_core_tools!(
            provider::anthropic_builder(fast_model, api_key)?
                .name("analyze_subagent")
                .preamble("You are a specialized analysis sub-agent.")
                .max_tokens(4096)
        )
        .build();

        // Build main agent with tools
        let builder = provider::anthropic_builder(&self.model, api_key)?
            .preamble(self.preamble.as_deref().unwrap_or("You are Iris."))
            .max_tokens(16384);

        let builder = crate::attach_core_tools!(builder)
            .tool(DebugTool::new(GitRepoInfo))
            .tool(DebugTool::new(self.workspace.clone()))
            .tool(DebugTool::new(ParallelAnalyze::with_timeout(
                &self.provider,
                fast_model,
                subagent_timeout,
                api_key,
            )?))
            .tool(sub_agent);

        // Conditionally attach content update tools for chat mode
        if let Some(sender) = &self.content_update_sender {
            use crate::agents::tools::{UpdateCommitTool, UpdatePRTool, UpdateReviewTool};
            Ok(builder
                .tool(DebugTool::new(UpdateCommitTool::new(sender.clone())))
                .tool(DebugTool::new(UpdatePRTool::new(sender.clone())))
                .tool(DebugTool::new(UpdateReviewTool::new(sender.clone())))
                .build())
        } else {
            Ok(builder.build())
        }
    }

    /// Build Gemini agent for streaming (with tools attached)
    fn build_gemini_agent_for_streaming(
        &self,
        _prompt: &str,
    ) -> Result<rig::agent::Agent<provider::GeminiModel>> {
        use crate::agents::debug_tool::DebugTool;

        let fast_model = self.effective_fast_model();
        let api_key = self.get_api_key();
        let subagent_timeout = self
            .config
            .as_ref()
            .map_or(120, |c| c.subagent_timeout_secs);

        // Build subagent
        let sub_agent = crate::attach_core_tools!(
            provider::gemini_builder(fast_model, api_key)?
                .name("analyze_subagent")
                .preamble("You are a specialized analysis sub-agent.")
                .max_tokens(4096)
        )
        .build();

        // Build main agent with tools
        let builder = provider::gemini_builder(&self.model, api_key)?
            .preamble(self.preamble.as_deref().unwrap_or("You are Iris."))
            .max_tokens(16384);

        let builder = crate::attach_core_tools!(builder)
            .tool(DebugTool::new(GitRepoInfo))
            .tool(DebugTool::new(self.workspace.clone()))
            .tool(DebugTool::new(ParallelAnalyze::with_timeout(
                &self.provider,
                fast_model,
                subagent_timeout,
                api_key,
            )?))
            .tool(sub_agent);

        // Conditionally attach content update tools for chat mode
        if let Some(sender) = &self.content_update_sender {
            use crate::agents::tools::{UpdateCommitTool, UpdatePRTool, UpdateReviewTool};
            Ok(builder
                .tool(DebugTool::new(UpdateCommitTool::new(sender.clone())))
                .tool(DebugTool::new(UpdatePRTool::new(sender.clone())))
                .tool(DebugTool::new(UpdateReviewTool::new(sender.clone())))
                .build())
        } else {
            Ok(builder.build())
        }
    }

    /// Load capability configuration from embedded TOML, returning both prompt and output type
    fn load_capability_config(&self, capability: &str) -> Result<(String, String)> {
        let _ = self; // Keep &self for method syntax consistency
        // Use embedded capability strings - always available regardless of working directory
        let content = match capability {
            "commit" => CAPABILITY_COMMIT,
            "pr" => CAPABILITY_PR,
            "review" => CAPABILITY_REVIEW,
            "changelog" => CAPABILITY_CHANGELOG,
            "release_notes" => CAPABILITY_RELEASE_NOTES,
            "chat" => CAPABILITY_CHAT,
            "semantic_blame" => CAPABILITY_SEMANTIC_BLAME,
            _ => {
                // Return generic prompt for unknown capabilities
                return Ok((
                    format!(
                        "You are helping with a {capability} task. Use the available Git tools to assist the user."
                    ),
                    "PlainText".to_string(),
                ));
            }
        };

        // Parse TOML to extract both task_prompt and output_type
        let parsed: toml::Value = toml::from_str(content)?;

        let task_prompt = parsed
            .get("task_prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No task_prompt found in capability file"))?;

        let output_type = parsed
            .get("output_type")
            .and_then(|v| v.as_str())
            .unwrap_or("PlainText")
            .to_string();

        Ok((task_prompt.to_string(), output_type))
    }

    /// Get the current capability being executed
    pub fn current_capability(&self) -> Option<&str> {
        self.current_capability.as_deref()
    }

    /// Simple single-turn execution for basic queries
    pub async fn chat(&self, message: &str) -> Result<String> {
        let agent = self.build_agent()?;
        let response = agent.prompt(message).await?;
        Ok(response)
    }

    /// Set the current capability
    pub fn set_capability(&mut self, capability: &str) {
        self.current_capability = Some(capability.to_string());
    }

    /// Get provider configuration
    pub fn provider_config(&self) -> &HashMap<String, String> {
        &self.provider_config
    }

    /// Set provider configuration
    pub fn set_provider_config(&mut self, config: HashMap<String, String>) {
        self.provider_config = config;
    }

    /// Set custom preamble
    pub fn set_preamble(&mut self, preamble: String) {
        self.preamble = Some(preamble);
    }

    /// Set configuration
    pub fn set_config(&mut self, config: crate::config::Config) {
        self.config = Some(config);
    }

    /// Set fast model for subagents
    pub fn set_fast_model(&mut self, fast_model: String) {
        self.fast_model = Some(fast_model);
    }
}

/// Builder for creating `IrisAgent` instances with different configurations
pub struct IrisAgentBuilder {
    provider: String,
    model: String,
    preamble: Option<String>,
}

impl IrisAgentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            preamble: None,
        }
    }

    /// Set the provider to use
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set a custom preamble
    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// Build the `IrisAgent`
    pub fn build(self) -> Result<IrisAgent> {
        let mut agent = IrisAgent::new(&self.provider, &self.model)?;

        // Apply custom preamble if provided
        if let Some(preamble) = self.preamble {
            agent.set_preamble(preamble);
        }

        Ok(agent)
    }
}

impl Default for IrisAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_json_response;
    use serde_json::Value;
    use std::borrow::Cow;

    #[test]
    fn sanitize_json_response_is_noop_for_valid_payloads() {
        let raw = r#"{"title":"Test","description":"All good"}"#;
        let sanitized = sanitize_json_response(raw);
        assert!(matches!(sanitized, Cow::Borrowed(_)));
        serde_json::from_str::<Value>(sanitized.as_ref()).expect("valid JSON");
    }

    #[test]
    fn sanitize_json_response_escapes_literal_newlines() {
        let raw = "{\"description\": \"Line1
Line2\"}";
        let sanitized = sanitize_json_response(raw);
        assert_eq!(sanitized.as_ref(), "{\"description\": \"Line1\\nLine2\"}");
        serde_json::from_str::<Value>(sanitized.as_ref()).expect("json sanitized");
    }
}
