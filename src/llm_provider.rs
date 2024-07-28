use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait LLMProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
}

pub struct OpenAIProvider {
    pub api_key: String,
}

pub struct ClaudeProvider {
    pub api_key: String,
}

// Implementations for OpenAIProvider and ClaudeProvider will be added in separate files