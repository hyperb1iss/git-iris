// src/provider_registry.rs
use crate::claude_provider::ClaudeProvider;
use crate::config::ProviderConfig;
use crate::llm_provider::LLMProvider;
use crate::openai_provider::OpenAIProvider;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Fn(ProviderConfig) -> Result<Arc<dyn LLMProvider>>>>,
    default_models: HashMap<String, &'static str>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = ProviderRegistry {
            providers: HashMap::new(),
            default_models: HashMap::new(),
        };
        registry.register("openai", |config| {
            Ok(Arc::new(OpenAIProvider::new(
                config.to_llm_provider_config(),
            )))
        });
        registry.register("claude", |config| {
            Ok(Arc::new(ClaudeProvider::new(
                config.to_llm_provider_config(),
            )))
        });
        registry.set_default_model("openai", OpenAIProvider::default_model());
        registry.set_default_model("claude", ClaudeProvider::default_model());
        registry
    }

    pub fn register<F>(&mut self, name: &str, creator: F)
    where
        F: Fn(ProviderConfig) -> Result<Arc<dyn LLMProvider>> + 'static,
    {
        self.providers.insert(name.to_string(), Box::new(creator));
    }

    pub fn set_default_model(&mut self, provider: &str, model: &'static str) {
        self.default_models.insert(provider.to_string(), model);
    }

    pub fn get_default_model(&self, provider: &str) -> Option<&'static str> {
        self.default_models.get(provider).copied()
    }

    pub fn create_provider(
        &self,
        name: &str,
        config: ProviderConfig,
    ) -> Result<Arc<dyn LLMProvider>> {
        if let Some(creator) = self.providers.get(name) {
            creator(config)
        } else {
            Err(anyhow!("Provider '{}' not found in registry", name))
        }
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
