use crate::llm_provider::{LLMProvider, OpenAIProvider};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let client = Client::new();

        let request_body = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
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
}
