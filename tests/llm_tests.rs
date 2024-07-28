use anyhow::Result;
use async_trait::async_trait;
use git_iris::config::{Config, ProviderConfig};
use git_iris::git::GitInfo;
use git_iris::llm::get_refined_message;
use git_iris::llm_provider::{LLMProvider, ProviderMap};
use mockall::mock;
use mockall::predicate::*;
use std::collections::HashMap;
use std::sync::Arc;

mock! {
    pub LLMProvider {}
    #[async_trait]
    impl LLMProvider for LLMProvider {
        async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
        fn is_unsupported(&self) -> bool;
        fn provider_name(&self) -> &str;
    }
}

fn create_dummy_git_info() -> GitInfo {
    GitInfo {
        branch: "main".to_string(),
        recent_commits: vec!["abc1234 Initial commit".to_string()],
        staged_files: {
            let mut map = HashMap::new();
            map.insert(
                "src/main.rs".to_string(),
                git_iris::FileChange {
                    status: "M".to_string(),
                    diff: "- old line\n+ new line".to_string(),
                },
            );
            map
        },
        unstaged_files: vec![],
        project_root: "/path/to/project".to_string(),
    }
}

fn create_dummy_config() -> Config {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: "dummy_openai_api_key".to_string(),
            model: "gpt-4o".to_string(),
            additional_params: HashMap::new(),
        },
    );
    providers.insert(
        "claude".to_string(),
        ProviderConfig {
            api_key: "dummy_claude_api_key".to_string(),
            model: "claude-3-sonnet".to_string(),
            additional_params: HashMap::new(),
        },
    );

    Config {
        default_provider: "openai".to_string(),
        providers,
        use_gitmoji: false,
        custom_instructions: String::new(),
    }
}

#[tokio::test]
async fn test_get_refined_message_openai() -> Result<()> {
    let git_info = create_dummy_git_info();
    let config = create_dummy_config();

    let mut mock_provider = MockLLMProvider::new();
    mock_provider
        .expect_generate_message()
        .with(always(), always())
        .times(1)
        .returning(|_, _| Ok("Mocked OpenAI response".to_string()));
    mock_provider.expect_is_unsupported().returning(|| false);
    mock_provider
        .expect_provider_name()
        .return_const("OpenAI".to_string());

    let mut providers = ProviderMap::new();
    providers.insert("openai".to_string(), Arc::new(mock_provider));
    git_iris::llm::init_providers(providers);

    let result = get_refined_message(&git_info, &config, "openai", false, false).await?;

    git_iris::llm::clear_providers();

    assert_eq!(result, "Mocked OpenAI response");

    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_claude() -> Result<()> {
    let git_info = create_dummy_git_info();
    let mut config = create_dummy_config();
    config.default_provider = "claude".to_string();

    let mut mock_provider = MockLLMProvider::new();
    mock_provider
        .expect_generate_message()
        .with(always(), always())
        .times(1)
        .returning(|_, _| Ok("Mocked Claude response".to_string()));
    mock_provider.expect_is_unsupported().returning(|| false);
    mock_provider
        .expect_provider_name()
        .return_const("Claude".to_string());

    let mut providers = ProviderMap::new();
    providers.insert("claude".to_string(), Arc::new(mock_provider));
    git_iris::llm::init_providers(providers);

    let result = get_refined_message(&git_info, &config, "claude", false, false).await?;

    git_iris::llm::clear_providers();

    assert_eq!(result, "Mocked Claude response");

    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_unsupported_provider() -> Result<()> {
    let git_info = create_dummy_git_info();
    let config = create_dummy_config();

    let result =
        get_refined_message(&git_info, &config, "unsupported_provider", false, false).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Provider 'unsupported_provider' not found in configuration"));

    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_with_gitmoji() -> Result<()> {
    let git_info = create_dummy_git_info();
    let mut config = create_dummy_config();
    config.use_gitmoji = true;

    let mut mock_provider = MockLLMProvider::new();
    mock_provider
        .expect_generate_message()
        .with(always(), always())
        .times(1)
        .returning(|system_prompt, _| {
            assert!(system_prompt.contains("Use a single gitmoji"));
            Ok("✨ Mocked response with gitmoji".to_string())
        });
    mock_provider.expect_is_unsupported().returning(|| false);
    mock_provider
        .expect_provider_name()
        .return_const("OpenAI".to_string());

    let mut providers = ProviderMap::new();
    providers.insert("openai".to_string(), Arc::new(mock_provider));
    git_iris::llm::init_providers(providers);

    let result = get_refined_message(&git_info, &config, "openai", true, false).await?;

    git_iris::llm::clear_providers();

    assert!(result.starts_with("✨"));

    Ok(())
}
