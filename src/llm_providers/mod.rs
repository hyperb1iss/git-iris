use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};
mod claude;
mod ollama;
mod openai;
mod test;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum LLMProviderType {
    OpenAI,
    Claude,
    Ollama,
    Test,
}

impl fmt::Display for LLMProviderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl FromStr for LLMProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LLMProviderType::OpenAI),
            "claude" => Ok(LLMProviderType::Claude),
            "ollama" => Ok(LLMProviderType::Ollama),
            "test" => Ok(LLMProviderType::Test),
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", s)),
        }
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
    pub requires_api_key: bool,
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
        LLMProviderType::Test => Ok(Box::new(test::TestLLMProvider::new(config)?)),
    }
}

pub fn get_provider_metadata(provider_type: &LLMProviderType) -> ProviderMetadata {
    match provider_type {
        LLMProviderType::OpenAI => openai::get_metadata(),
        LLMProviderType::Claude => claude::get_metadata(),
        LLMProviderType::Ollama => ollama::get_metadata(),
        LLMProviderType::Test => test::get_metadata(),
    }
}

pub fn get_available_providers() -> Vec<LLMProviderType> {
    LLMProviderType::iter().collect()
}

