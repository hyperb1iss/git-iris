use super::{LLMProvider, LLMProviderConfig};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

/// Represents the Ollama LLM provider
pub struct OllamaProvider {
    config: LLMProviderConfig,
    client: Client,
}

impl OllamaProvider {
    /// Creates a new instance of OllamaProvider with the given configuration
    pub fn new(config: LLMProviderConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: Client::new(),
        })
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    /// Generates a message using the Ollama API
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let request_body = json!({
            "model": self.config.model,
            "prompt": format!("{}\n\n{}", system_prompt, user_prompt),
            "stream": false
        });

        // Make the API request
        let response = self
            .client
            .post("http://localhost:11434/api/generate")
            .json(&request_body)
            .send()
            .await?;

        // Check for successful response
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Ollama API request failed with status {}: {}",
                status,
                text
            ));
        }

        // Parse the response body
        let response_body: serde_json::Value = response.json().await?;
        let content = response_body["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract content from Ollama API response"))?;

        Ok(content.to_string())
    }

    /// Returns the provider name
    fn provider_name() -> &'static str {
        "Ollama"
    }

    fn default_model() -> &'static str {
        "llama3"
    }

    fn default_token_limit() -> usize {
        100000
    }
}
