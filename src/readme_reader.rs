use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::llm::get_refined_message;
use crate::llm_providers::LLMProviderType;
use crate::{log_debug, Config};

pub fn find_and_read_readme(repo_path: &Path) -> Result<Option<String>> {
    let readme_patterns = ["README.md", "README.txt", "README", "Readme.md"];

    for pattern in readme_patterns.iter() {
        let readme_path = repo_path.join(pattern);
        if readme_path.exists() {
            let content = fs::read_to_string(readme_path.clone())?;
            log_debug!("Found README file: {:?}", readme_path);
            return Ok(Some(content));
        }
    }

    Ok(None)
}

pub async fn summarize_readme(
    config: &Config,
    provider_type: &LLMProviderType,
    readme_content: &str,
) -> Result<String> {
    let system_prompt =
        "You are an AI assistant tasked with summarizing README files for software projects. \
        Please provide a concise summary of the key points in the README, focusing on the project's
        purpose, main features, and any other crucial information.";

    let user_prompt = format!(
        "Please summarize the following README content:\n\n{}",
        readme_content
    );

    get_refined_message(config, provider_type, system_prompt, &user_prompt, None).await
}
