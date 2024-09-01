use anyhow::Result;
use git_iris::config::Config;
use git_iris::context::GeneratedMessage;
use git_iris::llm::{
    get_available_provider_names, get_default_model_for_provider,
    get_default_token_limit_for_provider, get_refined_message, get_refined_message_with_provider,
};
use git_iris::llm_providers::test::TestLLMProvider;
use git_iris::llm_providers::{LLMProviderConfig, LLMProviderType};
use std::collections::HashMap;
use std::str::FromStr;

#[tokio::test]
async fn test_get_refined_message_validating() -> Result<()> {
    let provider_config = LLMProviderConfig {
        api_key: String::new(),
        model: "test-model".to_string(),
        additional_params: HashMap::default(),
    };

    // Provide the provider response so we can test the validation logic
    let mut test_provider = TestLLMProvider::new(provider_config.clone());
    test_provider.set_response(
        "{\"emoji\": \"🚀\", \"title\": \"Test Title\", \"message\": \"Test Message\"}".to_string(),
    );

    // Call get_refined_message with the test provider
    let result: GeneratedMessage = get_refined_message_with_provider::<GeneratedMessage>(
        Box::new(test_provider.clone()),
        "System prompt including any custom instructions",
        "User prompt",
    )
    .await?;

    assert_eq!(result.emoji, Some("🚀".to_string()));
    assert_eq!(result.title, "Test Title");
    assert_eq!(result.message, "Test Message");
    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_with_preset_and_custom_instructions() -> Result<()> {
    let config = Config {
        default_provider: "test".to_string(),
        instruction_preset: "default".to_string(),
        instructions: "Custom instructions".to_string(),
        ..Default::default()
    };

    // Call get_refined_message with the test provider
    let result = get_refined_message::<String>(
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
    assert!(!providers.contains(&"test".to_string()));
}

#[test]
fn test_get_default_model_for_provider() -> Result<()> {
    assert_eq!(
        get_default_model_for_provider(&LLMProviderType::OpenAI),
        "gpt-4o"
    );
    assert_eq!(
        get_default_model_for_provider(&LLMProviderType::Claude),
        "claude-3-5-sonnet-20240620"
    );
    assert_eq!(
        get_default_model_for_provider(&LLMProviderType::Test),
        "test-model"
    );
    Ok(())
}

#[test]
fn test_get_default_token_limit_for_provider() -> Result<()> {
    assert_eq!(
        get_default_token_limit_for_provider(&LLMProviderType::OpenAI)?,
        100_000
    );
    assert_eq!(
        get_default_token_limit_for_provider(&LLMProviderType::Claude)?,
        150_000
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

#[tokio::test]
async fn test_get_refined_message_with_provider() -> Result<()> {
    git_iris::logger::init().expect("Failed to initialize logger");
    git_iris::logger::enable_logging();
    git_iris::logger::set_log_to_stdout(true);

    let provider_config = LLMProviderConfig {
        api_key: String::new(),
        model: "test-model".to_string(),
        additional_params: HashMap::default(),
    };

    // Test with 2 failures (should succeed on third try)
    let test_provider = TestLLMProvider::new(provider_config.clone());
    test_provider.set_fail_count(2);

    let result = get_refined_message_with_provider(
        Box::new(test_provider.clone()),
        "System prompt",
        "User prompt",
    )
    .await;

    println!("result 1: {result:?}");

    assert!(result.is_ok(), "Expected Ok result, got {result:?}");
    let message: String = result.unwrap();
    assert!(message.contains("Test response from model 'test-model'"));
    assert!(message.contains("System prompt:"));
    assert!(message.contains("User prompt:"));

    // We expect 3 total calls: 2 failures and 1 success
    assert_eq!(test_provider.get_total_calls(), 3);

    // Test immediate success
    let test_provider = TestLLMProvider::new(provider_config.clone());
    let result = get_refined_message_with_provider::<String>(
        Box::new(test_provider.clone()),
        "System prompt",
        "User prompt",
    )
    .await;

    println!("result 2: {result:?}");

    assert!(result.is_ok());
    assert_eq!(test_provider.get_total_calls(), 1);

    // Test with 3 failures (should fail after all retries)
    let test_provider = TestLLMProvider::new(provider_config.clone());
    test_provider.set_fail_count(3); // Set to 3 to ensure it always fails

    println!("start 3");
    let result = get_refined_message_with_provider::<String>(
        Box::new(test_provider.clone()),
        "System prompt",
        "User prompt",
    )
    .await;
    println!("result 3: {result:?}");

    assert!(result.is_err(), "Expected Err result, got {result:?}");
    // We expect 3 total calls: 3 failures
    assert_eq!(test_provider.get_total_calls(), 3);

    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_json_validation_failures() -> Result<()> {
    let provider_config = LLMProviderConfig {
        api_key: String::new(),
        model: "test-model".to_string(),
        additional_params: HashMap::default(),
    };

    // Provide the provider response so we can test the validation logic
    let mut test_provider = TestLLMProvider::new(provider_config.clone());
    test_provider.set_response(
        "{\"emoji\": \"🚀\", \"title\": \"Test Title\", \"message\": \"Test Message\"}".to_string(),
    );
    test_provider.set_bad_response("{\"invalid_key\": \"invalid_value\"}".to_string());
    test_provider.set_json_validation_failures(2);

    // Call get_refined_message with the test provider
    let result: GeneratedMessage = get_refined_message_with_provider::<GeneratedMessage>(
        Box::new(test_provider.clone()),
        "System prompt including any custom instructions",
        "User prompt",
    )
    .await?;

    assert_eq!(result.emoji, Some("🚀".to_string()));
    assert_eq!(result.title, "Test Title");
    assert_eq!(result.message, "Test Message");
    Ok(())
}
