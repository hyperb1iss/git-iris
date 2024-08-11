use crate::config::{Config, ProviderConfig};
use crate::llm_providers::{
    create_provider, get_available_providers, get_provider_metadata, LLMProviderConfig,
    LLMProviderType,
};
use crate::{log_debug, LLMProvider};
use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

/// Generates a message using the given configuration
pub async fn get_refined_message<T>(
    config: &Config,
    provider_type: &LLMProviderType,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<T>
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
    String: Into<T>,
{
    // Get provider metadata and configuration
    let provider_metadata = get_provider_metadata(provider_type);
    let provider_config = if provider_metadata.requires_api_key {
        config
            .get_provider_config(&provider_type.to_string())
            .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider_type))?
            .clone()
    } else {
        ProviderConfig::default_for(&provider_type.to_string())
    };

    // Create the LLM provider instance
    let llm_provider = create_provider(
        provider_type.clone(),
        provider_config.to_llm_provider_config(),
    )?;

    log_debug!(
        "Generating refined message using provider: {}",
        provider_type
    );
    log_debug!("System prompt: {}", system_prompt);
    log_debug!("User prompt: {}", user_prompt);

    // Call get_refined_message_with_provider
    let result = get_refined_message_with_provider::<T>(
        llm_provider,
        system_prompt,
        user_prompt,
    )
    .await?;

    Ok(result)
}

/// Generates a message using the given provider (mainly for testing purposes)
pub async fn get_refined_message_with_provider<T>(
    llm_provider: Box<dyn LLMProvider + Send + Sync>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<T>
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
    String: Into<T>,
{
    log_debug!("Entering get_refined_message_with_provider");

    let retry_strategy = ExponentialBackoff::from_millis(10).factor(2).take(2); // 2 attempts total: initial + 1 retry

    let result = Retry::spawn(retry_strategy, || async {
        log_debug!("Attempting to generate message");
        match tokio::time::timeout(
            Duration::from_secs(30),
            llm_provider.generate_message(system_prompt, user_prompt),
        )
        .await
        {
            Ok(Ok(refined_message)) => {
                log_debug!("Received response from provider");
                let cleaned_message = clean_json_from_llm(&refined_message);
                if std::any::type_name::<T>() == std::any::type_name::<String>() {
                    // If T is String, return the raw string response
                    Ok(cleaned_message.into())
                } else {
                    // Attempt to deserialize the response
                    match serde_json::from_str::<T>(&cleaned_message) {
                        Ok(message) => Ok(message),
                        Err(e) => {
                            log_debug!("Deserialization error: {}", e);
                            Err(anyhow!("Deserialization error: {}", e))
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                log_debug!("Provider error: {}", e);
                Err(e)
            }
            Err(_) => {
                log_debug!("Provider timed out");
                Err(anyhow!("Provider timed out"))
            }
        }
    })
    .await;

    match result {
        Ok(message) => {
            log_debug!("Deserialized message: {:?}", message);
            Ok(message)
        }
        Err(e) => {
            log_debug!("Failed to generate message after retries: {}", e);
            Err(anyhow!("Failed to generate message: {}", e))
        }
    }
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

fn clean_json_from_llm(json_str: &str) -> String {
    // Remove potential leading/trailing whitespace
    let trimmed = json_str.trim();

    // If wrapped in code block, remove the markers
    let without_codeblock = if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let start = trimmed.find('{').unwrap_or(0);
        let end = trimmed.rfind('}').map(|i| i + 1).unwrap_or(trimmed.len());
        &trimmed[start..end]
    } else {
        trimmed
    };

    // Find the first '{' and last '}' to extract the JSON object
    let start = without_codeblock.find('{').unwrap_or(0);
    let end = without_codeblock
        .rfind('}')
        .map(|i| i + 1)
        .unwrap_or(without_codeblock.len());

    without_codeblock[start..end].trim().to_string()
}
