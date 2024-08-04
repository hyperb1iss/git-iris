use super::{LLMProvider, LLMProviderConfig, ProviderMetadata};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

/// Represents the OpenAI LLM provider
pub struct OpenAIProvider {
    config: LLMProviderConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Creates a new instance of OpenAIProvider with the given configuration
    pub fn new(config: LLMProviderConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: Client::new(),
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    /// Generates a message using the OpenAI API
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let mut request_body = json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        });

        // Add additional parameters from the configuration
        for (key, value) in &self.config.additional_params {
            request_body[key] = serde_json::Value::String(value.clone());
        }

        // Make the API request
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        // Check for successful response
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "OpenAI API request failed with status {}: {}",
                status,
                text
            ));
        }

        // Parse the response body
        let response_body: serde_json::Value = response.json().await?;
        let content = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract content from OpenAI API response"))?;

        Ok(content.to_string())
    }
}

pub(super) fn get_metadata() -> ProviderMetadata {
    ProviderMetadata {
        name: "OpenAI",
        default_model: "gpt-4o",
        default_token_limit: 100000,
    }
}
