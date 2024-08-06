use crate::config::{Config, ProviderConfig};
use crate::llm_providers::{
    create_provider, get_available_providers, get_provider_metadata, LLMProviderConfig,
    LLMProviderType,
};
use crate::log_debug;
use anyhow::{anyhow, Result};

/// Generates a message using the given configuration
pub async fn get_refined_message(
    config: &Config,
    provider_type: &LLMProviderType,
    system_prompt: &str,
    user_prompt: &str,
    custom_instructions: Option<&str>,
) -> Result<String> {
    let provider_metadata = get_provider_metadata(provider_type);

    let provider_config = if provider_metadata.requires_api_key {
        config
            .get_provider_config(&provider_type.to_string())
            .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider_type))?
            .clone()
    } else {
        // Use default configuration for providers that don't require an API key
        ProviderConfig::default_for(&provider_type.to_string())
    };

    // Create the LLM provider instance using the provided function
    let llm_provider = create_provider(
        provider_type.clone(),
        provider_config.to_llm_provider_config(),
    )?;

    // Append custom instructions to the user prompt if provided
    let final_system_prompt = match custom_instructions {
        Some(instructions) => format!(
            "{}\n\nAdditional instructions: {}",
            system_prompt, instructions
        ),
        None => system_prompt.to_string(),
    };

    log_debug!(
        "Generating refined message using provider: {}",
        provider_type
    );
    log_debug!("System prompt: {}", final_system_prompt);
    log_debug!("User prompt: {}", user_prompt);

    // Generate the message using the LLM provider
    let refined_message = llm_provider
        .generate_message(&final_system_prompt, user_prompt)
        .await?;

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
        let provider_config = config
            .get_provider_config(&provider_type.to_string())
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
        model: get_default_model_for_provider(provider_type)
            .unwrap()
            .to_string(),
        additional_params: Default::default(),
    };

    let saved_config = config
        .get_provider_config(&provider_type.to_string())
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
