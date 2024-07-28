use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;
use crate::config::Config;

pub async fn get_refined_message(prompt: &str, use_gitmoji: bool, verbose: bool) -> Result<String> {
    let config = Config::load()?;
    let client = Client::new();

    let system_prompt = create_system_prompt(use_gitmoji);

    let request_body = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 150
    });

    if verbose {
        println!("Request to OpenAI API:\n{}", serde_json::to_string_pretty(&request_body)?);
    }

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request_body)
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

    if verbose {
        println!("Response from OpenAI API:\n{}", serde_json::to_string_pretty(&response_body)?);
    }

    let refined_message = response_body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to extract message from OpenAI API response. The response format might have changed."))?
        .trim()
        .to_string();

    Ok(refined_message)
}

fn create_system_prompt(use_gitmoji: bool) -> String {
    let mut prompt = String::from(
        "You are an AI assistant specialized in creating high-quality Git commit messages. \
        Your task is to generate clear, concise, and informative commit messages based on \
        the provided context. Follow these guidelines:

        1. Use the imperative mood in the subject line (e.g., 'Add feature' not 'Added feature').
        2. Limit the subject line to 50 characters if possible, but never exceed 72 characters.
        3. Capitalize the subject line.
        4. Do not end the subject line with a period.
        5. Separate subject from body with a blank line.
        6. Wrap the body at 72 characters.
        7. Use the body to explain what and why vs. how.
        8. If applicable, use semantic commit messages (e.g., feat:, fix:, docs:, style:, refactor:, test:, chore:).
        9. When multiple files or changes are involved, summarize the overall change in the subject line and use bullet points in the body for details.
        10. Be specific and avoid vague commit messages like 'Update file.txt' or 'Fix bug'.
        11. Do not use backticks around filenames in the messages.
        12. Focus on significant changes and avoid mentioning minor details like individual import additions.

        Remember, a good commit message should complete the following sentence:
        If applied, this commit will... <your subject line here>"
    );

    if use_gitmoji {
        prompt.push_str("\n\n13. Use appropriate gitmoji at the start of the commit message. \
        Choose the most relevant emoji and do not use more than one. \
        Some common gitmoji include:
        - ✨ (sparkles) for new features
        - 🐛 (bug) for bug fixes
        - 📚 (books) for documentation changes
        - 💄 (lipstick) for UI and style changes
        - ♻️ (recycle) for code refactoring
        - ✅ (white check mark) for adding tests
        - 🔧 (wrench) for configuration changes");
    }

    prompt
}