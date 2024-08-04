use crate::config::Config;
use crate::context::CommitContext;
use crate::llm_providers::LLMProviderConfig;
use crate::llm_providers::{create_provider, LLMProviderType};
use crate::log_debug;
use crate::prompt;
use crate::LLMProvider;
use anyhow::{anyhow, Result};

/// Generates a refined commit message using the specified LLM provider
pub async fn get_refined_message(
    context: &CommitContext,
    config: &Config,
    provider: &str,
    use_gitmoji: bool,
    verbose: bool,
    custom_instructions: &str,
) -> Result<String> {
    get_refined_message_with_provider(
        context,
        config,
        provider,
        use_gitmoji,
        verbose,
        custom_instructions,
        create_provider,
    )
    .await
}

/// Generates a refined commit message using the specified LLM provider
/// This version allows for a custom provider creation function, useful for testing
pub async fn get_refined_message_with_provider(
    context: &CommitContext,
    config: &Config,
    provider: &str,
    use_gitmoji: bool,
    verbose: bool,
    custom_instructions: &str,
    create_provider_fn: impl Fn(LLMProviderType, LLMProviderConfig) -> Result<Box<dyn LLMProvider>>,
) -> Result<String> {
    // Convert provider string to LLMProviderType
    let provider_type = LLMProviderType::from_str(provider)?;

    // Get provider configuration from the global config
    let provider_config = config
        .get_provider_config(provider)
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider))?;

    // Create the LLM provider instance using the provided function
    let llm_provider = create_provider_fn(provider_type, provider_config.to_llm_provider_config())?;

    // Generate system and user prompts
    let system_prompt = prompt::create_system_prompt(use_gitmoji, custom_instructions);
    let user_prompt = prompt::create_prompt(context, config, provider, verbose)?;

    log_debug!("Using LLM provider: {}", llm_provider.provider_name());

    // Generate the commit message using the LLM provider
    let refined_message = llm_provider
        .generate_message(&system_prompt, &user_prompt)
        .await?;

    // Log the generated message if verbose mode is enabled
    if verbose {
        log_debug!("Generated message:\n{}", refined_message);
    }

    Ok(refined_message)
}

/// Returns a list of available LLM providers
pub fn get_available_providers() -> Vec<String> {
    LLMProviderType::available_providers()
}

/// Returns the default model for a given provider
pub fn get_default_model_for_provider(provider: &str) -> Result<&'static str> {
    let provider_type = LLMProviderType::from_str(provider)?;
    Ok(crate::llm_providers::get_default_model(&provider_type))
}

/// Returns the default token limit for a given provider
pub fn get_default_token_limit_for_provider(provider: &str) -> Result<usize> {
    let provider_type = LLMProviderType::from_str(provider)?;
    Ok(crate::llm_providers::get_default_token_limit(
        &provider_type,
    ))
}
