use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
    fn is_unsupported(&self) -> bool {
        false
    }
    fn provider_name(&self) -> &str;
}

impl fmt::Display for dyn LLMProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.provider_name())
    }
}

pub struct OpenAIProvider {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate_message(&self, _system_prompt: &str, _user_prompt: &str) -> Result<String> {
        // Implementation for OpenAI
        todo!("Implement OpenAI message generation")
    }

    fn provider_name(&self) -> &str {
        "OpenAI"
    }
}

pub struct ClaudeProvider {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    async fn generate_message(&self, _system_prompt: &str, _user_prompt: &str) -> Result<String> {
        // Implementation for Claude
        todo!("Implement Claude message generation")
    }

    fn provider_name(&self) -> &str {
        "Claude"
    }
}

pub type ProviderMap = HashMap<String, Arc<dyn LLMProvider>>;
