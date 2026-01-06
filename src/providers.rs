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
}

impl Provider {
    /// All available providers
    pub const ALL: &'static [Provider] = &[Provider::OpenAI, Provider::Anthropic, Provider::Google];

    /// Provider name as used in config files and CLI
    pub const fn name(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Google => "google",
        }
    }

    /// Default model for complex analysis tasks
    pub const fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-5.1",
            Self::Anthropic => "claude-sonnet-4-5-20250929",
            Self::Google => "gemini-3-pro-preview",
        }
    }

    /// Fast model for simple tasks (status updates, parsing)
    pub const fn default_fast_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-5.1-mini",
            Self::Anthropic => "claude-haiku-4-5-20251001",
            Self::Google => "gemini-2.5-flash",
        }
    }

    /// Context window size (max tokens)
    pub const fn context_window(&self) -> usize {
        match self {
            Self::OpenAI => 128_000,
            Self::Anthropic => 200_000,
            Self::Google => 1_000_000,
        }
    }

    /// Environment variable name for the API key
    pub const fn api_key_env(&self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Google => "GOOGLE_API_KEY",
        }
    }

    /// Expected API key prefix for basic format validation
    ///
    /// Returns the expected prefix for the provider's API keys, if known.
    /// This helps catch typos and misconfigurations early.
    pub const fn api_key_prefix(&self) -> Option<&'static str> {
        match self {
            Self::OpenAI => Some("sk-"),
            Self::Anthropic => Some("sk-ant-"),
            Self::Google => None, // Google API keys don't have a consistent prefix
        }
    }

    /// Validate API key format
    ///
    /// Performs basic validation to catch obvious misconfigurations:
    /// - Checks for expected prefix (OpenAI: `sk-`, Anthropic: `sk-ant-`)
    /// - Ensures key is not suspiciously short
    ///
    /// Returns `Ok(())` if valid, or a warning message if potentially invalid.
    /// Note: A valid format doesn't guarantee the key works - it may still be
    /// expired or revoked. This just catches typos.
    pub fn validate_api_key_format(&self, key: &str) -> Result<(), String> {
        // Check minimum length (API keys are typically 30+ chars)
        if key.len() < 20 {
            return Err(format!(
                "{} API key appears too short (got {} chars, expected 20+)",
                self.name(),
                key.len()
            ));
        }

        // Check expected prefix
        if let Some(prefix) = self.api_key_prefix() {
            if !key.starts_with(prefix) {
                return Err(format!(
                    "{} API key should start with '{}', got '{}'",
                    self.name(),
                    prefix,
                    &key[..key.len().min(6)] // Show first 6 chars safely
                ));
            }
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
        // Handle legacy "claude" alias
        let normalized = if lower == "claude" {
            "anthropic"
        } else {
            &lower
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
    #[error("Unknown provider: {0}. Supported: openai, anthropic, google")]
    Unknown(String),
    #[error("API key required for provider: {0}")]
    MissingApiKey(String),
}

/// Per-provider configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Token limit override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_limit: Option<usize>,
    /// Additional provider-specific params
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_params: HashMap<String, String>,
}

impl ProviderConfig {
    /// Create config with defaults for a provider
    pub fn with_defaults(provider: Provider) -> Self {
        Self {
            api_key: String::new(),
            model: provider.default_model().to_string(),
            fast_model: Some(provider.default_fast_model().to_string()),
            token_limit: None,
            additional_params: HashMap::new(),
        }
    }

    /// Get effective model (configured or default)
    pub fn effective_model(&self, provider: Provider) -> &str {
        if self.model.is_empty() {
            provider.default_model()
        } else {
            &self.model
        }
    }

    /// Get effective fast model (configured or default)
    pub fn effective_fast_model(&self, provider: Provider) -> &str {
        self.fast_model
            .as_deref()
            .unwrap_or_else(|| provider.default_fast_model())
    }

    /// Get effective token limit (configured or default)
    pub fn effective_token_limit(&self, provider: Provider) -> usize {
        self.token_limit
            .unwrap_or_else(|| provider.context_window())
    }

    /// Check if this config has an API key set
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
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
        assert!("invalid".parse::<Provider>().is_err());
    }

    #[test]
    fn test_provider_defaults() {
        assert_eq!(Provider::OpenAI.default_model(), "gpt-5.1");
        assert_eq!(Provider::Anthropic.context_window(), 200_000);
        assert_eq!(Provider::Google.api_key_env(), "GOOGLE_API_KEY");
    }

    #[test]
    fn test_provider_config_defaults() {
        let config = ProviderConfig::with_defaults(Provider::Anthropic);
        assert_eq!(config.model, "claude-sonnet-4-5-20250929");
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
    fn test_api_key_validation_valid_openai() {
        // Valid OpenAI key format (starts with sk-, long enough)
        let result = Provider::OpenAI.validate_api_key_format("sk-1234567890abcdefghijklmnop");
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
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_api_key_validation_wrong_prefix_openai() {
        // Long enough but wrong prefix
        let result = Provider::OpenAI.validate_api_key_format("wrong-prefix-1234567890abcdef");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("should start with"));
    }

    #[test]
    fn test_api_key_validation_wrong_prefix_anthropic() {
        // Has sk- but not sk-ant- (might be OpenAI key used for Anthropic)
        let result = Provider::Anthropic.validate_api_key_format("sk-1234567890abcdefghijklmnop");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sk-ant-"));
    }
}
