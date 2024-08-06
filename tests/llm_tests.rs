use anyhow::Result;
use git_iris::config::Config;
use git_iris::llm::{
    get_available_provider_names, get_default_model_for_provider,
    get_default_token_limit_for_provider, get_refined_message,
};
use git_iris::llm_providers::LLMProviderType;

#[tokio::test]
async fn test_get_refined_message() -> Result<()> {
    let mut config = Config::default();
    config.default_provider = "test".to_string();

    // Call get_refined_message_with_provider with the test provider
    let result = get_refined_message(
        &config,
        &LLMProviderType::OpenAI,
        "System prompt",
        "User prompt",
        None,
    )
    .await?;
    assert_eq!(result, "Mocked commit message");
    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_with_custom_instructions() -> Result<()> {
    let mut config = Config::default();
    config.default_provider = "test".to_string();

    // Call get_refined_message_with_provider with the mock provider and custom instructions
    let result = get_refined_message(
        &config,
        &LLMProviderType::OpenAI,
        "System prompt",
        "User prompt",
        Some("Custom instruction"),
    )
    .await?;
    assert_eq!(result, "Mocked commit message with custom instruction");
    Ok(())
}

#[test]
fn test_get_available_providers() {
    let providers = get_available_provider_names();
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"claude".to_string()));
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
    assert!(LLMProviderType::from_str("invalid").is_err());
}
