use crate::llm_provider::{LLMProvider, LLMProviderConfig};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

pub struct OpenAIProvider {
    config: LLMProviderConfig,
}

impl OpenAIProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self { config }
    }

    pub fn default_model() -> &'static str {
        "gpt-4o"
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let client = Client::new();

        let mut request_body = json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        });

        // Add additional parameters
        for (key, value) in &self.config.additional_params {
            request_body[key] = serde_json::Value::String(value.clone());
        }

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "OpenAI API request failed with status {}: {}",
                status,
                text
            ));
        }

        let response_body: serde_json::Value = response.json().await?;
        let message = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract message from OpenAI API response"))?
            .trim()
            .to_string();

        Ok(message)
    }

    fn provider_name(&self) -> &str {
        "OpenAI"
    }
}