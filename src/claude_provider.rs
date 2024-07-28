use crate::llm_provider::{ClaudeProvider, LLMProvider};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

#[async_trait]
impl LLMProvider for ClaudeProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let client = Client::new();

        let request_body = json!({
            "model": "claude-3-5-sonnet",
            "prompt": format!("Human: {}\n\nHuman: {}\n\nAssistant:", system_prompt, user_prompt),
        });

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
}
