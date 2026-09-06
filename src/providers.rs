//! LLM Provider configuration.
//!
//! Single source of truth for supported providers and their defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    OpenAI,
    Anthropic,
    Google,
    OpenRouter,
    Fireworks,
}

impl Provider {
    /// All available providers
    pub const ALL: &'static [Provider] = &[
        Provider::OpenAI,
        Provider::Anthropic,
        Provider::Google,
        Provider::OpenRouter,
        Provider::Fireworks,
    ];

    /// Provider name as used in config files and CLI
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Google => "google",
            Self::OpenRouter => "openrouter",
            Self::Fireworks => "fireworks",
        }
    }

    /// Default model for complex analysis tasks
    #[must_use]
    pub const fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-6-astra",
            Self::Anthropic => "claude-opus-5",
            Self::Google => "gemini-3.8-flash",
            Self::OpenRouter => "anthropic/claude-opus-5",
            Self::Fireworks => "accounts/fireworks/models/deepseek-v4-pro-0813",
        }
    }

    /// Fast model for simple tasks (status updates, parsing)
    #[must_use]
    pub const fn default_fast_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-5.6-luna",
            Self::Anthropic => "claude-haiku-4-5-20251001",
            Self::Google => "gemini-3.5-flash-lite",
            Self::OpenRouter => "anthropic/claude-haiku-4.5",
            Self::Fireworks => "accounts/fireworks/models/deepseek-v4-flash-0731",
        }
    }

    /// Context window size (max tokens)
    #[must_use]
    pub const fn context_window(&self) -> usize {
        match self {
            Self::OpenAI => 1_050_000,
            Self::Anthropic | Self::OpenRouter | Self::Fireworks => 1_000_000,
            Self::Google => 1_048_576,
        }
    }

    /// Environment variable name for the API key
    #[must_use]
    pub const fn api_key_env(&self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Google => "GOOGLE_API_KEY",
            Self::OpenRouter => "OPENROUTER_API_KEY",
            Self::Fireworks => "FIREWORKS_API_KEY",
        }
    }

    /// Valid API key prefixes for format validation
    ///
    /// Returns the expected prefixes for the provider's API keys.
    /// `OpenAI` has multiple valid prefixes (sk-, sk-proj-).
    #[must_use]
    pub fn api_key_prefixes(&self) -> &'static [&'static str] {
        match self {
            Self::OpenAI => &["sk-", "sk-proj-"],
            Self::Anthropic => &["sk-ant-"],
            Self::OpenRouter => &["sk-or-"],
            Self::Google | Self::Fireworks => &[], // Google API keys don't have a consistent prefix
        }
    }

    /// Expected API key prefix for basic format validation (primary prefix)
    ///
    /// Returns the primary expected prefix for display in error messages.
    #[must_use]
    pub const fn api_key_prefix(&self) -> Option<&'static str> {
        match self {
            Self::OpenAI => Some("sk-"),
            Self::Anthropic => Some("sk-ant-"),
            Self::OpenRouter => Some("sk-or-"),
            Self::Google | Self::Fireworks => None,
        }
    }

    /// Validate API key format
    ///
    /// Performs basic validation to catch obvious misconfigurations:
    /// - Checks for expected prefix (`OpenAI`: `sk-` or `sk-proj-`, `Anthropic`: `sk-ant-`)
    /// - Ensures key is not suspiciously short
    ///
    /// Returns `Ok(())` if valid, or a warning message if potentially invalid.
    /// Note: A valid format doesn't guarantee the key works - it may still be
    /// expired or revoked. This just catches typos.
    ///
    /// # Errors
    ///
    /// Returns an error string when the key format is clearly invalid for the provider.
    pub fn validate_api_key_format(&self, key: &str) -> Result<(), String> {
        // Check minimum length (API keys are typically 30+ chars)
        if key.len() < 20 {
            return Err(format!(
                "{} API key appears too short (got {} chars, expected 20+)",
                self.name(),
                key.len()
            ));
        }

        // Check expected prefixes
        let prefixes = self.api_key_prefixes();
        if !prefixes.is_empty() && !prefixes.iter().any(|p| key.starts_with(p)) {
            let expected = if prefixes.len() == 1 {
                format!("'{}'", prefixes[0])
            } else {
                prefixes
                    .iter()
                    .map(|p| format!("'{p}'"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            };
            return Err(format!(
                "{} API key should start with {} (key has unexpected prefix)",
                self.name(),
                expected
            ));
        }

        Ok(())
    }

    /// Get all provider names as strings
    pub fn all_names() -> Vec<&'static str> {
        Self::ALL.iter().map(Self::name).collect()
    }
}

impl FromStr for Provider {
    type Err = ProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        // Handle legacy/common aliases
        let normalized = match lower.as_str() {
            "claude" => "anthropic",
            "gemini" => "google",
            _ => &lower,
        };

        Self::ALL
            .iter()
            .find(|p| p.name() == normalized)
            .copied()
            .ok_or_else(|| ProviderError::Unknown(s.to_string()))
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Provider configuration error
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Unknown provider: {0}. Supported: openai, anthropic, google, openrouter, fireworks")]
    Unknown(String),
    #[error("API key required for provider: {0}")]
    MissingApiKey(String),
}

/// Per-provider configuration
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API key (loaded from env or config)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_key: String,
    /// Primary model for complex analysis
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Fast model for simple tasks
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast_model: Option<String>,
    /// Model for delegated analysis (defaults to the primary model)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_model: Option<String>,
    /// Token limit override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_limit: Option<usize>,
    /// Additional provider-specific params
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_params: HashMap<String, String>,
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderConfig")
            .field(
                "api_key",
                if self.api_key.is_empty() {
                    &"<empty>"
                } else {
                    &"[REDACTED]"
                },
            )
            .field("model", &self.model)
            .field("fast_model", &self.fast_model)
            .field("subagent_model", &self.subagent_model)
            .field("token_limit", &self.token_limit)
            .field("additional_params", &self.additional_params)
            .finish()
    }
}

impl ProviderConfig {
    /// Create config with defaults for a provider
    #[must_use]
    pub fn with_defaults(provider: Provider) -> Self {
        Self {
            api_key: String::new(),
            model: provider.default_model().to_string(),
            fast_model: Some(provider.default_fast_model().to_string()),
            subagent_model: None,
            token_limit: None,
            additional_params: HashMap::new(),
        }
    }

    /// Get effective model (configured or default)
    #[must_use]
    pub fn effective_model(&self, provider: Provider) -> &str {
        if self.model.is_empty() {
            provider.default_model()
        } else {
            &self.model
        }
    }

    /// Get effective fast model (configured or default)
    #[must_use]
    pub fn effective_fast_model(&self, provider: Provider) -> &str {
        self.fast_model
            .as_deref()
            .unwrap_or_else(|| provider.default_fast_model())
    }

    /// Get the model used for delegated analysis.
    #[must_use]
    pub fn effective_subagent_model(&self, provider: Provider) -> &str {
        self.subagent_model
            .as_deref()
            .filter(|model| !model.is_empty())
            .unwrap_or_else(|| self.effective_model(provider))
    }

    /// Get effective token limit (configured or default)
    #[must_use]
    pub fn effective_token_limit(&self, provider: Provider) -> usize {
        self.token_limit
            .unwrap_or_else(|| provider.context_window())
    }

    /// Check if this config has an API key set
    #[must_use]
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Get API key if set (non-empty), otherwise None
    ///
    /// This is the canonical way to extract an API key for passing to
    /// provider builders. Returns `None` for empty strings, allowing
    /// fallback to environment variables.
    #[must_use]
    pub fn api_key_if_set(&self) -> Option<&str> {
        if self.api_key.is_empty() {
            None
        } else {
            Some(&self.api_key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!("openai".parse::<Provider>().ok(), Some(Provider::OpenAI));
        assert_eq!(
            "ANTHROPIC".parse::<Provider>().ok(),
            Some(Provider::Anthropic)
        );
        assert_eq!("claude".parse::<Provider>().ok(), Some(Provider::Anthropic)); // Legacy alias
        assert_eq!("gemini".parse::<Provider>().ok(), Some(Provider::Google)); // Common alias
        assert!("invalid".parse::<Provider>().is_err());
    }

    #[test]
    fn test_provider_defaults() {
        assert_eq!(Provider::OpenAI.default_model(), "gpt-6-astra");
        assert_eq!(Provider::OpenAI.default_fast_model(), "gpt-5.6-luna");
        assert_eq!(Provider::Anthropic.context_window(), 1_000_000);
        assert_eq!(Provider::Google.api_key_env(), "GOOGLE_API_KEY");
    }

    #[test]
    fn test_provider_config_defaults() {
        let config = ProviderConfig::with_defaults(Provider::Anthropic);
        assert_eq!(config.model, "claude-opus-5");
        assert_eq!(
            config.fast_model.as_deref(),
            Some("claude-haiku-4-5-20251001")
        );
    }

    #[test]
    fn test_api_key_prefix() {
        assert_eq!(Provider::OpenAI.api_key_prefix(), Some("sk-"));
        assert_eq!(Provider::Anthropic.api_key_prefix(), Some("sk-ant-"));
        assert_eq!(Provider::Google.api_key_prefix(), None);
    }

    #[test]
    fn test_api_key_if_set() {
        // Non-empty key returns Some
        let mut config = ProviderConfig::with_defaults(Provider::OpenAI);
        config.api_key = "sk-test-key-12345678901234567890".to_string();
        assert_eq!(
            config.api_key_if_set(),
            Some("sk-test-key-12345678901234567890")
        );

        // Empty key returns None
        config.api_key = String::new();
        assert_eq!(config.api_key_if_set(), None);
    }

    #[test]
    fn test_api_key_prefixes() {
        // OpenAI accepts multiple prefixes
        assert_eq!(Provider::OpenAI.api_key_prefixes(), &["sk-", "sk-proj-"]);
        assert_eq!(Provider::Anthropic.api_key_prefixes(), &["sk-ant-"]);
        assert!(Provider::Google.api_key_prefixes().is_empty());
    }

    #[test]
    fn test_api_key_validation_valid_openai() {
        // Valid OpenAI key format (starts with sk-, long enough)
        let result = Provider::OpenAI.validate_api_key_format("sk-1234567890abcdefghijklmnop");
        assert!(result.is_ok());
    }

    #[test]
    fn test_api_key_validation_valid_openai_project_key() {
        // Valid OpenAI project key format (starts with sk-proj-, long enough)
        let result = Provider::OpenAI.validate_api_key_format("sk-proj-1234567890abcdefghijklmnop");
        assert!(result.is_ok());
    }

    #[test]
    fn test_api_key_validation_valid_anthropic() {
        // Valid Anthropic key format (starts with sk-ant-, long enough)
        let result =
            Provider::Anthropic.validate_api_key_format("sk-ant-1234567890abcdefghijklmnop");
        assert!(result.is_ok());
    }

    #[test]
    fn test_api_key_validation_valid_google() {
        // Google keys don't have a prefix requirement, just length
        let result = Provider::Google.validate_api_key_format("AIzaSyA1234567890abcdefgh");
        assert!(result.is_ok());
    }

    #[test]
    fn test_api_key_validation_too_short() {
        let result = Provider::OpenAI.validate_api_key_format("sk-short");
        assert!(result.is_err());
        assert!(result.expect_err("should be err").contains("too short"));
    }

    #[test]
    fn test_api_key_validation_wrong_prefix_openai() {
        // Long enough but wrong prefix
        let result = Provider::OpenAI.validate_api_key_format("wrong-prefix-1234567890abcdef");
        assert!(result.is_err());
        let err = result.expect_err("should be err");
        assert!(err.contains("should start with"));
        // Error should mention valid prefixes
        assert!(err.contains("'sk-'") || err.contains("'sk-proj-'"));
        // Verify we don't expose the actual key prefix in error messages
        assert!(!err.contains("wrong-"));
    }

    #[test]
    fn test_api_key_validation_wrong_prefix_anthropic() {
        // Has sk- but not sk-ant- (might be OpenAI key used for Anthropic)
        let result = Provider::Anthropic.validate_api_key_format("sk-1234567890abcdefghijklmnop");
        assert!(result.is_err());
        let err = result.expect_err("should be err");
        assert!(err.contains("sk-ant-"));
        // Verify we don't expose the actual key content
        assert!(err.contains("unexpected prefix"));
    }
}
