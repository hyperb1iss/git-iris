use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};
mod claude;
mod ollama;
mod openai;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum LLMProviderType {
    OpenAI,
    Claude,
    Ollama,
}

impl fmt::Display for LLMProviderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
}

pub struct ProviderMetadata {
    pub name: &'static str,
    pub default_model: &'static str,
    pub default_token_limit: usize,
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
        LLMProviderType::Ollama => Ok(Box::new(ollama::OllamaProvider::new(config)?)),
    }
}

pub fn get_provider_metadata(provider_type: &LLMProviderType) -> ProviderMetadata {
    match provider_type {
        LLMProviderType::OpenAI => openai::get_metadata(),
        LLMProviderType::Claude => claude::get_metadata(),
        LLMProviderType::Ollama => ollama::get_metadata(),
    }
}

pub fn get_available_providers() -> Vec<LLMProviderType> {
    LLMProviderType::iter().collect()
}

impl LLMProviderType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LLMProviderType::OpenAI),
            "claude" => Ok(LLMProviderType::Claude),
            "ollama" => Ok(LLMProviderType::Ollama),
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", s)),
        }
    }
}
