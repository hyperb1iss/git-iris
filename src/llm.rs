use crate::claude_provider::ClaudeProvider;
use crate::config::Config;
use crate::git::GitInfo;
use crate::llm_provider::LLMProvider;
use crate::openai_provider::OpenAIProvider;
use crate::prompt;
use anyhow::{anyhow, Result};

pub async fn get_refined_message(
    git_info: &GitInfo,
    config: &Config,
    provider: &str,
    use_gitmoji: bool,
    verbose: bool,
) -> Result<String> {
    let provider_config = config
        .get_provider_config(provider)
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider))?;

    let provider: Box<dyn LLMProvider> = match provider {
        "openai" => Box::new(OpenAIProvider {
            api_key: provider_config.api_key.clone(),
            model: provider_config.model.clone(),
            additional_params: provider_config.additional_params.clone(),
        }),
        "claude" => Box::new(ClaudeProvider {
            api_key: provider_config.api_key.clone(),
            model: provider_config.model.clone(),
            additional_params: provider_config.additional_params.clone(),
        }),
        _ => return Err(anyhow!("Unsupported LLM provider: {}", provider)),
    };

    let system_prompt = prompt::create_system_prompt(use_gitmoji, &config.custom_instructions);
    let user_prompt = prompt::create_user_prompt(git_info, verbose)?;

    if verbose {
        println!("Using LLM provider: {}", provider);
        println!("System prompt:\n{}", system_prompt);
        println!("User prompt:\n{}", user_prompt);
    }

    let refined_message = provider
        .generate_message(&system_prompt, &user_prompt)
        .await?;

    if verbose {
        println!("Generated message:\n{}", refined_message);
    }

    Ok(refined_message)
}
