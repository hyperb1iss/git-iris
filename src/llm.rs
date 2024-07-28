use crate::config::Config;
use crate::llm_provider::{LLMProvider, OpenAIProvider, ClaudeProvider};
use crate::prompt;
use crate::git::GitInfo;
use anyhow::Result;

pub async fn get_refined_message(git_info: &GitInfo, config: &Config, use_gitmoji: bool, verbose: bool) -> Result<String> {
    let provider: Box<dyn LLMProvider> = match config.llm_provider.as_str() {
        "openai" => Box::new(OpenAIProvider { api_key: config.api_key.clone() }),
        "claude" => Box::new(ClaudeProvider { api_key: config.api_key.clone() }),
        _ => return Err(anyhow::anyhow!("Unsupported LLM provider: {}", config.llm_provider)),
    };

    let system_prompt = prompt::create_system_prompt(use_gitmoji, &config.custom_instructions);
    let user_prompt = prompt::create_user_prompt(git_info, verbose)?;

    if verbose {
        println!("Using LLM provider: {}", config.llm_provider);
        println!("System prompt:\n{}", system_prompt);
        println!("User prompt:\n{}", user_prompt);
    }

    let refined_message = provider.generate_message(&system_prompt, &user_prompt).await?;

    if verbose {
        println!("Generated message:\n{}", refined_message);
    }

    Ok(refined_message)
}