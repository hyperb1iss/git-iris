//! Dynamic provider abstraction for rig-core 0.27+
//!
//! This module provides runtime provider selection using enum dispatch,
//! allowing git-iris to work with any supported provider based on config.

use rig::{
    agent::{Agent, AgentBuilder, PromptResponse},
    client::{CompletionClient, ProviderClient},
    completion::{Prompt, PromptError},
    providers::{anthropic, gemini, openai},
};

use crate::providers::Provider;

/// Completion model types for each provider
pub type OpenAIModel = openai::completion::CompletionModel;
pub type AnthropicModel = anthropic::completion::CompletionModel;
pub type GeminiModel = gemini::completion::CompletionModel;

/// Agent builder types for each provider
pub type OpenAIBuilder = AgentBuilder<OpenAIModel>;
pub type AnthropicBuilder = AgentBuilder<AnthropicModel>;
pub type GeminiBuilder = AgentBuilder<GeminiModel>;

/// Dynamic agent that can be any provider's agent type
pub enum DynAgent {
    OpenAI(Agent<OpenAIModel>),
    Anthropic(Agent<AnthropicModel>),
    Gemini(Agent<GeminiModel>),
}

impl DynAgent {
    /// Simple prompt - returns response string
    pub async fn prompt(&self, msg: &str) -> Result<String, PromptError> {
        match self {
            Self::OpenAI(a) => a.prompt(msg).await,
            Self::Anthropic(a) => a.prompt(msg).await,
            Self::Gemini(a) => a.prompt(msg).await,
        }
    }

    /// Multi-turn prompt with specified depth for tool calling
    pub async fn prompt_multi_turn(&self, msg: &str, depth: usize) -> Result<String, PromptError> {
        match self {
            Self::OpenAI(a) => a.prompt(msg).multi_turn(depth).await,
            Self::Anthropic(a) => a.prompt(msg).multi_turn(depth).await,
            Self::Gemini(a) => a.prompt(msg).multi_turn(depth).await,
        }
    }

    /// Multi-turn prompt with extended details (token usage, etc.)
    pub async fn prompt_extended(
        &self,
        msg: &str,
        depth: usize,
    ) -> Result<PromptResponse, PromptError> {
        match self {
            Self::OpenAI(a) => a.prompt(msg).multi_turn(depth).extended_details().await,
            Self::Anthropic(a) => a.prompt(msg).multi_turn(depth).extended_details().await,
            Self::Gemini(a) => a.prompt(msg).multi_turn(depth).extended_details().await,
        }
    }
}

/// Resolve API key from config or environment variable.
///
/// Tries config first, then falls back to the provider's environment variable.
/// Returns `None` if neither source has a key (will cause `from_env()` to be used,
/// which may panic - caller should validate beforehand).
fn resolve_api_key(api_key: Option<&str>, provider: Provider) -> Option<String> {
    // If explicit key provided and non-empty, use it
    if let Some(key) = api_key {
        if !key.is_empty() {
            return Some(key.to_string());
        }
    }
    // Fall back to environment variable
    std::env::var(provider.api_key_env()).ok()
}

/// Create an `OpenAI` agent builder
///
/// # Arguments
/// * `model` - The model name to use
/// * `api_key` - Optional API key. If not provided, falls back to `OPENAI_API_KEY` env var.
///
/// # Panics
/// Panics if no API key is available (neither in config nor env var).
pub fn openai_builder(model: &str, api_key: Option<&str>) -> OpenAIBuilder {
    let client = match resolve_api_key(api_key, Provider::OpenAI) {
        Some(key) => openai::Client::new(&key).expect("Failed to create OpenAI client"),
        None => openai::Client::from_env(),
    };
    client.completions_api().agent(model)
}

/// Create an Anthropic agent builder
///
/// # Arguments
/// * `model` - The model name to use
/// * `api_key` - Optional API key. If not provided, falls back to `ANTHROPIC_API_KEY` env var.
///
/// # Panics
/// Panics if no API key is available (neither in config nor env var).
pub fn anthropic_builder(model: &str, api_key: Option<&str>) -> AnthropicBuilder {
    let client = match resolve_api_key(api_key, Provider::Anthropic) {
        Some(key) => anthropic::Client::new(&key).expect("Failed to create Anthropic client"),
        None => anthropic::Client::from_env(),
    };
    client.agent(model)
}

/// Create a Gemini agent builder
///
/// # Arguments
/// * `model` - The model name to use
/// * `api_key` - Optional API key. If not provided, falls back to `GOOGLE_API_KEY` env var.
///
/// # Panics
/// Panics if no API key is available (neither in config nor env var).
pub fn gemini_builder(model: &str, api_key: Option<&str>) -> GeminiBuilder {
    let client = match resolve_api_key(api_key, Provider::Google) {
        Some(key) => gemini::Client::new(&key).expect("Failed to create Gemini client"),
        None => gemini::Client::from_env(),
    };
    client.agent(model)
}
