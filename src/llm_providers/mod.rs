use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use strum_macros::{EnumIter, AsRefStr};
use strum::IntoEnumIterator;

pub mod openai;
pub mod claude;

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
    fn provider_name(&self) -> &'static str;
    fn default_model(&self) -> &'static str;
    fn default_token_limit(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct LLMProviderConfig {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

pub fn create_provider(provider_type: LLMProviderType, config: LLMProviderConfig) -> Result<Box<dyn LLMProvider>> {
    match provider_type {
        LLMProviderType::OpenAI => Ok(Box::new(openai::OpenAIProvider::new(config)?)),
        LLMProviderType::Claude => Ok(Box::new(claude::ClaudeProvider::new(config)?)),
    }
}

pub fn get_default_model(provider_type: &LLMProviderType) -> &'static str {
    match provider_type {
        LLMProviderType::OpenAI => "gpt-4o",
        LLMProviderType::Claude => "claude-3-5-sonnet-20240620",
    }
}

pub fn get_default_token_limit(provider_type: &LLMProviderType) -> usize {
    match provider_type {
        LLMProviderType::OpenAI => 100000,
        LLMProviderType::Claude => 150000,
    }
}