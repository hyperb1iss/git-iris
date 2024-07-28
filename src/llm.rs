use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub async fn get_refined_message(prompt: &str) -> Result<String> {
    let client = Client::new();
    let api_key = std::env::var("OPENAI_API_KEY")?;

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant that refines Git commit messages."},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 100
        }))
        .send()
        .await?;

    let response_body: serde_json::Value = response.json().await?;
    let refined_message = response_body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse LLM response"))?
        .trim()
        .to_string();

    Ok(refined_message)
}