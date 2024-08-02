use crate::log_debug;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Trait defining the interface for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a message based on the system and user prompts
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
    /// Check if the provider is unsupported
    fn is_unsupported(&self) -> bool {
        false
    }
    /// Get the name of the provider
    fn provider_name(&self) -> &str;
    fn default_token_limit(&self) -> usize {
        4000 // Default fallback value
    }
}

/// Manager for handling multiple LLM providers
pub struct LLMProviderManager {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
}

impl LLMProviderManager {
    /// Create a new LLMProviderManager
    pub fn new() -> Self {
        LLMProviderManager {
            providers: HashMap::new(),
        }
    }

    /// Register a new LLM provider
    pub fn register_provider(&mut self, name: String, provider: Arc<dyn LLMProvider>) {
        self.providers.insert(name.clone(), provider);
        log_debug!("Registered provider: {}", name);
    }

    /// Get a registered LLM provider by name
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(name)
    }

    /// Clear all registered providers
    pub fn clear_providers(&mut self) {
        self.providers.clear();
        log_debug!("Cleared all registered providers");
    }
}

impl fmt::Display for dyn LLMProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.provider_name())
    }
}

/// Configuration structure for an LLM provider
#[derive(Debug, Clone)]
pub struct LLMProviderConfig {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

/// Implementation of OpenAI provider
pub struct OpenAIProvider {
    pub(crate) _config: LLMProviderConfig,
}

/// Implementation of Claude provider
pub struct ClaudeProvider {
    pub(crate) _config: LLMProviderConfig,
}

/// Type alias for a map of provider names to provider instances
pub type ProviderMap = HashMap<String, Arc<dyn LLMProvider>>;
