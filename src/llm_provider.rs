use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;

#[async_trait]
pub trait LLMProvider: fmt::Display {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
}

pub struct OpenAIProvider {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

impl fmt::Display for OpenAIProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpenAI")
    }
}

pub struct ClaudeProvider {
    pub api_key: String,
    pub model: String,
    pub additional_params: HashMap<String, String>,
}

impl fmt::Display for ClaudeProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Claude")
    }
}

// Implementations for OpenAIProvider and ClaudeProvider will be in separate files
