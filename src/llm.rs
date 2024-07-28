use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;
use crate::config::Config;

pub async fn get_refined_message(prompt: &str) -> Result<String> {
    let config = Config::load()?;
    let client = Client::new();

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant that refines Git commit messages."},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 100
        }))
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send request to OpenAI API: {}. Please check your internet connection and API key.", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        return Err(anyhow!("OpenAI API request failed with status {}: {}. Please check your API key and usage limits.", status, text));
    }

    let response_body: serde_json::Value = response.json().await
        .map_err(|e| anyhow!("Failed to parse OpenAI API response: {}. This might be due to an API change or temporary service issue.", e))?;

    let refined_message = response_body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to extract message from OpenAI API response. The response format might have changed."))?
        .trim()
        .to_string();

    Ok(refined_message)
}