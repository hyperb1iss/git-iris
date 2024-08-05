use crate::config::Config;
use crate::context::CommitContext;
use crate::llm_providers::{
    create_provider, get_available_providers, get_provider_metadata, LLMProviderConfig,
    LLMProviderType,
};
use crate::log_debug;
use crate::prompt;
use crate::LLMProvider;
use anyhow::{anyhow, Result};

/// Generates a refined commit message using the specified LLM provider
pub async fn get_refined_message(
    context: &CommitContext,
    config: &Config,
    provider_type: &LLMProviderType,
    use_gitmoji: bool,
    custom_instructions: &str,
) -> Result<String> {
    get_refined_message_with_provider(
        context,
        config,
        provider_type,
        use_gitmoji,
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
    provider_type: &LLMProviderType,
    use_gitmoji: bool,
    custom_instructions: &str,
    create_provider_fn: impl Fn(LLMProviderType, LLMProviderConfig) -> Result<Box<dyn LLMProvider>>,
) -> Result<String> {
    // Get provider configuration from the global config
    let provider_config = config
        .get_provider_config(&provider_type.to_string())
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider_type))?;

    // Create the LLM provider instance using the provided function
    let llm_provider = create_provider_fn(provider_type.clone(), provider_config.to_llm_provider_config())?;

    // Generate system and user prompts
    let system_prompt = prompt::create_system_prompt(use_gitmoji, custom_instructions);
    let user_prompt = prompt::create_user_prompt(context)?;

    log_debug!("Using LLM provider: {}", provider_type);

    // Generate the commit message using the LLM provider
    let refined_message = llm_provider
        .generate_message(&system_prompt, &user_prompt)
        .await?;

    log_debug!("Generated message:\n{}", refined_message);

    Ok(refined_message)
}

/// Returns a list of available LLM providers as strings
pub fn get_available_provider_names() -> Vec<String> {
    get_available_providers()
        .into_iter()
        .map(|p| p.to_string())
        .collect()
}

/// Returns the default model for a given provider
pub fn get_default_model_for_provider(provider_type: &LLMProviderType) -> Result<&'static str> {
    Ok(get_provider_metadata(provider_type).default_model)
}

/// Returns the default token limit for a given provider
pub fn get_default_token_limit_for_provider(provider_type: &LLMProviderType) -> Result<usize> {
    Ok(get_provider_metadata(provider_type).default_token_limit)
}

/// Checks if a provider requires an API key
pub fn provider_requires_api_key(provider_type: &LLMProviderType) -> bool {
    get_provider_metadata(provider_type).requires_api_key
}

/// Validates the provider configuration
pub fn validate_provider_config(config: &Config, provider_type: &LLMProviderType) -> Result<()> {
    let metadata = get_provider_metadata(provider_type);
    
    if metadata.requires_api_key {
        let provider_config = config.get_provider_config(&provider_type.to_string())
            .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider_type))?;

        if provider_config.api_key.is_empty() {
            return Err(anyhow!("API key required for provider: {}", provider_type));
        }
    }

    Ok(())
}

/// Combines default, saved, and command-line configurations
pub fn get_combined_config(
    config: &Config,
    provider_type: &LLMProviderType,
    command_line_args: &LLMProviderConfig,
) -> LLMProviderConfig {
    let default_config = LLMProviderConfig {
        api_key: String::new(),
        model: get_default_model_for_provider(provider_type).unwrap().to_string(),
        additional_params: Default::default(),
    };

    let saved_config = config.get_provider_config(&provider_type.to_string())
        .cloned()
        .unwrap_or_default();

    LLMProviderConfig {
        api_key: if !command_line_args.api_key.is_empty() {
            command_line_args.api_key.clone()
        } else if !saved_config.api_key.is_empty() {
            saved_config.api_key
        } else {
            default_config.api_key
        },
        model: if !command_line_args.model.is_empty() {
            command_line_args.model.clone()
        } else if !saved_config.model.is_empty() {
            saved_config.model
        } else {
            default_config.model
        },
        additional_params: if !command_line_args.additional_params.is_empty() {
            command_line_args.additional_params.clone()
        } else if !saved_config.additional_params.is_empty() {
            saved_config.additional_params
        } else {
            default_config.additional_params
        },
    }
}