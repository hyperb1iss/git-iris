use anyhow::Result;
use git_iris::config::Config;
use git_iris::llm::{
    get_available_provider_names, get_default_model_for_provider,
    get_default_token_limit_for_provider, get_refined_message,
};
use git_iris::llm_providers::LLMProviderType;
use std::str::FromStr;

#[tokio::test]
async fn test_get_refined_message() -> Result<()> {
    let mut config = Config::default();
    config.default_provider = "test".to_string();

    // Call get_refined_message with the test provider
    let result = get_refined_message(
        &config,
        &LLMProviderType::Test,
        "System prompt including any custom instructions",
        "User prompt",
    )
    .await?;
    assert!(result.contains("Test response from model 'test-model'"));
    assert!(result.contains("System prompt: 'System prompt including any custom instructions'"));
    assert!(result.contains("User prompt: 'User prompt'"));
    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_with_preset_and_custom_instructions() -> Result<()> {
    let mut config = Config::default();
    config.default_provider = "test".to_string();
    config.instruction_preset = "default".to_string();
    config.instructions = "Custom instructions".to_string();

    // Call get_refined_message with the test provider
    let result = get_refined_message(
        &config,
        &LLMProviderType::Test,
        "System prompt with preset: default and custom instructions: Custom instructions",
        "User prompt",
    )
    .await?;
    assert!(result.contains("Test response from model 'test-model'"));
    assert!(result.contains("System prompt: 'System prompt with preset: default and custom instructions: Custom instructions'"));
    assert!(result.contains("User prompt: 'User prompt'"));
    Ok(())
}

#[test]
fn test_get_available_providers() {
    let providers = get_available_provider_names();
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"claude".to_string()));
    assert!(providers.contains(&"test".to_string()));
}

#[test]
fn test_get_default_model_for_provider() -> Result<()> {
    assert_eq!(
        get_default_model_for_provider(&LLMProviderType::OpenAI)?,
        "gpt-4o"
    );
    assert_eq!(
        get_default_model_for_provider(&LLMProviderType::Claude)?,
        "claude-3-5-sonnet-20240620"
    );
    assert_eq!(
        get_default_model_for_provider(&LLMProviderType::Test)?,
        "test-model"
    );
    Ok(())
}

#[test]
fn test_get_default_token_limit_for_provider() -> Result<()> {
    assert_eq!(
        get_default_token_limit_for_provider(&LLMProviderType::OpenAI)?,
        100000
    );
    assert_eq!(
        get_default_token_limit_for_provider(&LLMProviderType::Claude)?,
        150000
    );
    assert_eq!(
        get_default_token_limit_for_provider(&LLMProviderType::Test)?,
        1000
    );
    Ok(())
}

#[test]
fn test_llm_provider_type_from_str() {
    assert_eq!(
        LLMProviderType::from_str("openai").unwrap(),
        LLMProviderType::OpenAI
    );
    assert_eq!(
        LLMProviderType::from_str("claude").unwrap(),
        LLMProviderType::Claude
    );
    assert_eq!(
        LLMProviderType::from_str("test").unwrap(),
        LLMProviderType::Test
    );
    assert!(LLMProviderType::from_str("invalid").is_err());
}
