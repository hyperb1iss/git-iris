use crate::llm_provider::{LLMProvider, LLMProviderConfig};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

/// Represents the Claude LLM provider
pub struct ClaudeProvider {
    config: LLMProviderConfig,
}

impl ClaudeProvider {
    /// Creates a new instance of ClaudeProvider with the given configuration
    pub fn new(config: LLMProviderConfig) -> Self {
        Self { config }
    }

    /// Returns the default model name for the Claude provider
    pub fn default_model() -> &'static str {
        "claude-3-5-sonnet-20240620"
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    /// Generates a message using the Claude API
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let client = Client::new();

        let mut request_body = json!({
            "model": self.config.model,
            "system": system_prompt, // Top-level system parameter
            "messages": [
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": 4096,
        });

        // Add additional parameters from the configuration
        for (key, value) in &self.config.additional_params {
            request_body[key] = serde_json::Value::String(value.clone());
        }

        // Make the API request
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        // Check for successful response
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Claude API request failed with status {}: {}",
                status,
                text
            ));
        }

        // Parse the response body
        let response_body: serde_json::Value = response.json().await?;
        let content_array = response_body["content"].as_array().ok_or_else(|| {
            anyhow::anyhow!("Failed to extract content array from Claude API response")
        })?;

        // Extract the message content
        let message = content_array
            .iter()
            .filter_map(|item| item["text"].as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        Ok(message)
    }

    /// Returns the provider name
    fn provider_name(&self) -> &str {
        "Claude"
    }
}
