//! Provider transports and workflow defaults for the shared Rig agent runtime.

use anyhow::Result;
use rig::{
    agent::{Agent, AgentBuilder, PromptResponse},
    client::{AgentClientExt, CompletionClient},
    completion::{Prompt, PromptError},
    providers::{anthropic, gemini, openai, openrouter},
};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

use crate::providers::{Provider, ProviderConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionProfile {
    MainAgent,
    Subagent,
    StatusMessage,
}

impl CompletionProfile {
    const fn default_openai_reasoning_effort(self) -> &'static str {
        match self {
            Self::MainAgent => "medium",
            Self::Subagent => "low",
            Self::StatusMessage => "none",
        }
    }
}

/// Shared provider-independent agent runtime.
#[derive(Clone)]
pub struct DynAgent(pub Agent);

impl DynAgent {
    /// Send a prompt through the selected provider.
    ///
    /// # Errors
    /// Returns the underlying provider or tool error.
    pub async fn prompt(&self, msg: &str) -> Result<String, PromptError> {
        self.0.prompt(msg).await
    }

    /// Execute a bounded multi-turn tool loop.
    ///
    /// # Errors
    /// Returns the underlying provider or tool error.
    pub async fn prompt_multi_turn(&self, msg: &str, depth: usize) -> Result<String, PromptError> {
        self.0.prompt(msg).max_turns(depth).await
    }

    /// Execute a tool loop and include usage details.
    ///
    /// # Errors
    /// Returns the underlying provider or tool error.
    pub async fn prompt_extended(
        &self,
        msg: &str,
        depth: usize,
    ) -> Result<PromptResponse, PromptError> {
        self.0.prompt(msg).max_turns(depth).extended_details().await
    }
}

/// Select the provider transport while sharing the agent runtime.
///
/// # Errors
/// Returns an error when provider credentials or client configuration are invalid.
pub fn agent_builder(
    provider: Provider,
    model: &str,
    api_key: Option<&str>,
) -> Result<AgentBuilder> {
    agent_builder_at(provider, model, api_key, None)
}

fn agent_builder_at(
    provider: Provider,
    model: &str,
    api_key: Option<&str>,
    base_url: Option<&str>,
) -> Result<AgentBuilder> {
    let key = required_api_key(api_key, provider)?;
    macro_rules! client {
        ($client:path) => {{
            let builder = <$client>::builder().api_key(&key);
            let builder = if let Some(url) = base_url {
                builder.base_url(url)
            } else {
                builder
            };
            builder.build().map_err(|_| {
                anyhow::anyhow!(
                    "Failed to create {provider} client: authentication or configuration error"
                )
            })?
        }};
    }
    Ok(match provider {
        Provider::OpenAI => client!(openai::Client).agent(model),
        Provider::Anthropic => anthropic_agent_builder(&client!(anthropic::Client), model),
        Provider::Google => client!(gemini::Client).agent(model),
        Provider::OpenRouter => client!(openrouter::Client).agent(model),
        Provider::Fireworks => {
            let base_url = base_url.unwrap_or(FIREWORKS_BASE_URL);
            let client = openai::Client::builder()
                .api_key(&key)
                .base_url(base_url)
                .build()
                .map_err(|_| anyhow::anyhow!("Failed to create Fireworks client"))?;
            client.completions_api().agent(model)
        }
    })
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
pub fn resolve_api_key(
    api_key: Option<&str>,
    provider: Provider,
) -> (Option<String>, ApiKeySource) {
    // If explicit key provided and non-empty, use it
    if let Some(key) = api_key
        && !key.is_empty()
    {
        tracing::trace!(
            provider = %provider,
            source = "config",
            "Using API key from configuration"
        );
        validate_and_warn(key, provider, "config");
        return (Some(key.to_string()), ApiKeySource::Config);
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

/// Enable Anthropic prompt caching for the complete multi-turn transcript.
pub fn anthropic_agent_builder(client: &anthropic::Client, model: &str) -> AgentBuilder {
    AgentBuilder::new(client.completion_model(model).with_automatic_caching())
}

pub(crate) const FIREWORKS_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";

fn required_api_key(api_key: Option<&str>, provider: Provider) -> Result<String> {
    resolve_api_key(api_key, provider)
        .0
        .filter(|key| !key.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "API key required for {provider}: set {} or configure a key",
                provider.api_key_env()
            )
        })
}

fn parse_additional_param_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn additional_params_json<S>(
    additional_params: Option<&HashMap<String, String, S>>,
) -> Map<String, Value>
where
    S: std::hash::BuildHasher,
{
    let mut params = Map::new();
    if let Some(additional_params) = additional_params {
        for (key, value) in additional_params {
            params.insert(key.clone(), parse_additional_param_value(value));
        }
    }
    params
}

fn completion_params_json<S>(
    additional_params: Option<&HashMap<String, String, S>>,
    provider: Provider,
    model: &str,
    _max_tokens: u64,
    profile: CompletionProfile,
) -> Map<String, Value>
where
    S: std::hash::BuildHasher,
{
    let mut params = additional_params_json(additional_params);
    let model = model.to_lowercase();
    match provider {
        Provider::OpenAI if model.starts_with("gpt-5") || model.starts_with("gpt-6") => {
            let effort =
                if model.starts_with("gpt-6") && profile == CompletionProfile::StatusMessage {
                    "low"
                } else {
                    profile.default_openai_reasoning_effort()
                };
            params
                .entry("reasoning")
                .or_insert_with(|| json!({"effort": effort}));
        }
        Provider::Anthropic if model.starts_with("claude-opus-5") => {
            if profile != CompletionProfile::StatusMessage {
                params
                    .entry("thinking")
                    .or_insert_with(|| json!({"type": "adaptive"}));
                let effort = if profile == CompletionProfile::MainAgent {
                    "high"
                } else {
                    "low"
                };
                let output = params.entry("output_config").or_insert_with(|| json!({}));
                if let Some(output) = output.as_object_mut() {
                    output.entry("effort").or_insert_with(|| json!(effort));
                }
            }
        }
        Provider::OpenRouter => {
            if model.starts_with("anthropic/claude-opus-5") {
                let effort = if profile == CompletionProfile::MainAgent {
                    "high"
                } else {
                    "low"
                };
                params
                    .entry("reasoning")
                    .or_insert_with(|| json!({"effort": effort}));
            } else if model.starts_with("openai/gpt-5") || model.starts_with("openai/gpt-6") {
                let effort = if model.starts_with("openai/gpt-6")
                    && profile == CompletionProfile::StatusMessage
                {
                    "low"
                } else {
                    profile.default_openai_reasoning_effort()
                };
                params
                    .entry("reasoning")
                    .or_insert_with(|| json!({"effort": effort}));
            }
        }
        Provider::Fireworks if model.starts_with("accounts/fireworks/models/deepseek-v4-") => {
            let effort = if profile == CompletionProfile::StatusMessage {
                "none"
            } else {
                "high"
            };
            if !params.contains_key("thinking") {
                params
                    .entry("reasoning_effort")
                    .or_insert_with(|| json!(effort));
            }
        }
        Provider::Google if model.starts_with("gemini-3.8-flash") => {
            let effort = if profile == CompletionProfile::MainAgent {
                "medium"
            } else {
                "low"
            };
            let config = params
                .entry("generationConfig")
                .or_insert_with(|| json!({}));
            if let Some(config) = config.as_object_mut() {
                config
                    .entry("thinkingConfig")
                    .or_insert_with(|| json!({"thinkingLevel": effort}));
            }
        }
        _ => {}
    }
    params
}

pub fn apply_completion_params<M, S>(
    builder: AgentBuilder<M>,
    provider: Provider,
    model: &str,
    max_tokens: u64,
    additional_params: Option<&HashMap<String, String, S>>,
    profile: CompletionProfile,
) -> AgentBuilder<M>
where
    S: std::hash::BuildHasher,
{
    let builder = builder.max_tokens(max_tokens);
    let params = completion_params_json(additional_params, provider, model, max_tokens, profile);
    if params.is_empty() {
        builder
    } else {
        builder.additional_params(Value::Object(params))
    }
}

/// Parse a configured provider name into the canonical provider enum.
///
/// # Errors
///
/// Returns an error when the provider name is not supported.
pub fn provider_from_name(provider: &str) -> Result<Provider> {
    provider
        .parse()
        .map_err(|_| anyhow::anyhow!("Unsupported provider: {}", provider))
}

#[must_use]
pub fn current_provider_config<'a>(
    config: Option<&'a crate::config::Config>,
    provider: &str,
) -> Option<&'a ProviderConfig> {
    config.and_then(|config| config.get_provider_config(provider))
}

#[cfg(test)]
mod tests;
