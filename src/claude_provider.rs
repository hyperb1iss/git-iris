use crate::llm_provider::{LLMProvider, ClaudeProvider, LLMProviderConfig};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

impl ClaudeProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self { config }
    }

    pub fn default_model() -> &'static str {
        "claude-3-5-sonnet-20240620"
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
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

        // Add additional parameters
        for (key, value) in &self.config.additional_params {
            request_body[key] = serde_json::Value::String(value.clone());
        }

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Claude API request failed with status {}: {}",
                status,
                text
            ));
        }

        let response_body: serde_json::Value = response.json().await?;
        let content_array = response_body["content"].as_array().ok_or_else(|| {
            anyhow::anyhow!("Failed to extract content array from Claude API response")
        })?;

        let message = content_array
            .iter()
            .filter_map(|item| item["text"].as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        Ok(message)
    }

    fn provider_name(&self) -> &str {
        "Claude"
    }
}