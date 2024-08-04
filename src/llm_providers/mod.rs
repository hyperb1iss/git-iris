use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

mod claude;
mod ollama;
mod openai;

#[derive(Debug, Clone, PartialEq, EnumIter, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum LLMProviderType {
    OpenAI,
    Claude,
}

impl LLMProviderType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LLMProviderType::OpenAI),
            "claude" => Ok(LLMProviderType::Claude),
            _ => Err(anyhow!("Unsupported provider: {}", s)),
        }
    }

    pub fn available_providers() -> Vec<String> {
        Self::iter().map(|p| p.as_ref().to_string()).collect()
    }
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
    fn provider_name() -> &'static str;
    fn default_model() -> &'static str;
    fn default_token_limit() -> usize;
}

#[derive(Clone, Debug)]
pub struct LLMProviderConfig {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

pub fn create_provider(
    provider_type: LLMProviderType,
    config: LLMProviderConfig,
) -> Result<Box<dyn LLMProvider>> {
    match provider_type {
        LLMProviderType::OpenAI => Ok(Box::new(openai::OpenAIProvider::new(config)?)),
        LLMProviderType::Claude => Ok(Box::new(claude::ClaudeProvider::new(config)?)),
    }
}

pub fn get_default_model(provider_type: &LLMProviderType) -> &'static str {
    match provider_type {
        LLMProviderType::OpenAI => openai::OpenAIProvider::default_model(),
        LLMProviderType::Claude => claude::ClaudeProvider::default_model(),
    }
}

pub fn get_default_token_limit(provider_type: &LLMProviderType) -> usize {
    match provider_type {
        LLMProviderType::OpenAI => openai::OpenAIProvider::default_token_limit(),
        LLMProviderType::Claude => claude::ClaudeProvider::default_token_limit(),
    }
}
