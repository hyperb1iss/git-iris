//! Dynamic provider abstraction for rig-core 0.27+
//!
//! This module provides runtime provider selection using enum dispatch,
//! allowing git-iris to work with any supported provider based on config.

use anyhow::{Context, Result};
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

/// Source of the resolved API key (for logging/debugging)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeySource {
    Config,
    Environment,
    ClientDefault,
}

/// Validate API key format and log warnings for suspicious keys
fn validate_and_warn(key: &str, provider: Provider, source: &str) {
    if let Err(warning) = provider.validate_api_key_format(key) {
        tracing::warn!(
            provider = %provider,
            source = source,
            "API key format warning: {}",
            warning
        );
    }
}

/// Resolve API key from config or environment variable.
///
/// Resolution order:
/// 1. If `api_key` is `Some` and non-empty, use it (from config)
/// 2. Otherwise, check the provider's environment variable
/// 3. If neither has a key, returns `None` (caller will use `from_env()`)
///
/// Note: An empty string in config is treated as "not configured" and falls
/// back to the environment variable. This allows users to override env vars
/// in config while still supporting env-only setups.
pub fn resolve_api_key(api_key: Option<&str>, provider: Provider) -> (Option<String>, ApiKeySource) {
    // If explicit key provided and non-empty, use it
    if let Some(key) = api_key {
        if !key.is_empty() {
            tracing::trace!(
                provider = %provider,
                source = "config",
                "Using API key from configuration"
            );
            validate_and_warn(key, provider, "config");
            return (Some(key.to_string()), ApiKeySource::Config);
        }
    }

    // Fall back to environment variable
    if let Ok(key) = std::env::var(provider.api_key_env()) {
        tracing::trace!(
            provider = %provider,
            env_var = %provider.api_key_env(),
            source = "environment",
            "Using API key from environment variable"
        );
        validate_and_warn(&key, provider, "environment");
        return (Some(key), ApiKeySource::Environment);
    }

    tracing::trace!(
        provider = %provider,
        source = "client_default",
        "No API key found, will use client's from_env()"
    );
    (None, ApiKeySource::ClientDefault)
}

/// Create an `OpenAI` agent builder
///
/// # Arguments
/// * `model` - The model name to use
/// * `api_key` - Optional API key from config. Resolution order:
///   1. Non-empty `api_key` parameter (from config)
///   2. `OPENAI_API_KEY` environment variable
///   3. Client's `from_env()` (requires env var to be set)
///
/// # Errors
/// Returns an error if client creation fails (invalid credentials or missing env var).
pub fn openai_builder(model: &str, api_key: Option<&str>) -> Result<OpenAIBuilder> {
    let (resolved_key, _source) = resolve_api_key(api_key, Provider::OpenAI);
    let client = match resolved_key {
        Some(key) => openai::Client::new(&key)
            .context("Failed to create OpenAI client with provided credentials")?,
        None => openai::Client::from_env(),
    };
    Ok(client.completions_api().agent(model))
}

/// Create an Anthropic agent builder
///
/// # Arguments
/// * `model` - The model name to use
/// * `api_key` - Optional API key from config. Resolution order:
///   1. Non-empty `api_key` parameter (from config)
///   2. `ANTHROPIC_API_KEY` environment variable
///   3. Client's `from_env()` (requires env var to be set)
///
/// # Errors
/// Returns an error if client creation fails (invalid credentials or missing env var).
pub fn anthropic_builder(model: &str, api_key: Option<&str>) -> Result<AnthropicBuilder> {
    let (resolved_key, _source) = resolve_api_key(api_key, Provider::Anthropic);
    let client = match resolved_key {
        Some(key) => anthropic::Client::new(&key)
            .context("Failed to create Anthropic client with provided credentials")?,
        None => anthropic::Client::from_env(),
    };
    Ok(client.agent(model))
}

/// Create a Gemini agent builder
///
/// # Arguments
/// * `model` - The model name to use
/// * `api_key` - Optional API key from config. Resolution order:
///   1. Non-empty `api_key` parameter (from config)
///   2. `GOOGLE_API_KEY` environment variable
///   3. Client's `from_env()` (requires env var to be set)
///
/// # Errors
/// Returns an error if client creation fails (invalid credentials or missing env var).
pub fn gemini_builder(model: &str, api_key: Option<&str>) -> Result<GeminiBuilder> {
    let (resolved_key, _source) = resolve_api_key(api_key, Provider::Google);
    let client = match resolved_key {
        Some(key) => gemini::Client::new(&key)
            .context("Failed to create Gemini client with provided credentials")?,
        None => gemini::Client::from_env(),
    };
    Ok(client.agent(model))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_api_key_uses_config_when_provided() {
        // Config key takes precedence
        let (key, source) =
            resolve_api_key(Some("sk-config-key-1234567890"), Provider::OpenAI);
        assert_eq!(key, Some("sk-config-key-1234567890".to_string()));
        assert_eq!(source, ApiKeySource::Config);
    }

    #[test]
    fn test_resolve_api_key_empty_config_not_used() {
        // Empty config should NOT be treated as a valid key
        // It should fall through to env var or client default
        let empty_config: Option<&str> = Some("");
        let (_key, source) = resolve_api_key(empty_config, Provider::OpenAI);

        // Empty config should NOT return Config source
        // This test verifies the empty string is treated as "not configured"
        assert_ne!(source, ApiKeySource::Config);
    }

    #[test]
    fn test_resolve_api_key_none_config_checks_env() {
        // When config is None, should check env var
        let (key, source) = resolve_api_key(None, Provider::OpenAI);

        // Result depends on whether OPENAI_API_KEY is set in the environment
        // We just verify the function doesn't panic and returns appropriate source
        match source {
            ApiKeySource::Environment => {
                assert!(key.is_some());
            }
            ApiKeySource::ClientDefault => {
                assert!(key.is_none());
            }
            ApiKeySource::Config => {
                panic!("Should not return Config source when config is None");
            }
        }
    }

    #[test]
    fn test_api_key_source_enum_equality() {
        assert_eq!(ApiKeySource::Config, ApiKeySource::Config);
        assert_eq!(ApiKeySource::Environment, ApiKeySource::Environment);
        assert_eq!(ApiKeySource::ClientDefault, ApiKeySource::ClientDefault);
        assert_ne!(ApiKeySource::Config, ApiKeySource::Environment);
    }
}
