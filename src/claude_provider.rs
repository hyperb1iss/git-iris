use crate::llm_provider::LLMProvider;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::fmt;

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

#[async_trait]
impl LLMProvider for ClaudeProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let client = Client::new();

        let mut request_body = json!({
            "model": self.model,
            "prompt": format!("Human: {}\n\nHuman: {}\n\nAssistant:", system_prompt, user_prompt),
        });

        // Add additional parameters
        for (key, value) in &self.additional_params {
            request_body[key] = serde_json::Value::String(value.clone());
        }

        let response = client
            .post("https://api.anthropic.com/v1/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        let message = response_body["completion"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract message from Claude API response"))?
            .trim()
            .to_string();

        Ok(message)
    }

    fn default_model(&self) -> &'static str {
        "claude-3-sonnet"
    }
}
