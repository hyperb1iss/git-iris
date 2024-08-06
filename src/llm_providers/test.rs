use super::{LLMProvider, LLMProviderConfig, ProviderMetadata};
use anyhow::Result;
use async_trait::async_trait;

/// Represents the Test LLM provider for use in testing
pub struct TestLLMProvider {
    config: LLMProviderConfig,
}

impl TestLLMProvider {
    /// Creates a new instance of TestLLMProvider with the given configuration
    pub fn new(config: LLMProviderConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl LLMProvider for TestLLMProvider {
    /// Generates a message using the Test provider (returns the model name as the message)
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        Ok(format!(
            "Test response from model '{}'. System prompt: '{}', User prompt: '{}'",
            self.config.model, system_prompt, user_prompt
        ))
    }
}

pub(super) fn get_metadata() -> ProviderMetadata {
    ProviderMetadata {
        name: "Test",
        default_model: "test-model",
        default_token_limit: 1000,
        requires_api_key: false,
    }
}
