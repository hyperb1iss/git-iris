use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use git_iris::config::{Config, ProviderConfig};
use git_iris::git::{FileChange, GitInfo};
use git_iris::llm::{clear_providers, get_refined_message, init_providers};
use git_iris::llm_provider::LLMProvider;

struct MockOpenAIProvider;

#[async_trait]
impl LLMProvider for MockOpenAIProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        Ok(format!(
            "Mock OpenAI response for: {} {}",
            system_prompt, user_prompt
        ))
    }

    fn provider_name(&self) -> &str {
        "MockOpenAI"
    }
}

struct MockClaudeProvider;

#[async_trait]
impl LLMProvider for MockClaudeProvider {
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        Ok(format!(
            "Mock Claude response for: {} {}",
            system_prompt, user_prompt
        ))
    }

    fn provider_name(&self) -> &str {
        "MockClaude"
    }
}

fn setup_mock_providers() {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        Arc::new(MockOpenAIProvider) as Arc<dyn LLMProvider>,
    );
    providers.insert(
        "claude".to_string(),
        Arc::new(MockClaudeProvider) as Arc<dyn LLMProvider>,
    );
    init_providers(providers);
}

fn teardown_mock_providers() {
    clear_providers();
}

fn create_mock_git_info() -> GitInfo {
    GitInfo {
        branch: "main".to_string(),
        recent_commits: vec!["abcdef1 Initial commit".to_string()],
        staged_files: {
            let mut map = HashMap::new();
            map.insert(
                "file1.rs".to_string(),
                FileChange {
                    status: "M".to_string(),
                    diff: "- old line\n+ new line".to_string(),
                },
            );
            map
        },
        unstaged_files: vec!["unstaged_file.txt".to_string()],
        project_root: "/mock/path/to/project".to_string(),
    }
}

fn create_default_config(provider: &str) -> Config {
    let mut providers = HashMap::new();
    providers.insert(
        provider.to_string(),
        ProviderConfig {
            api_key: "mock_key".to_string(),
            model: "mock-model".to_string(),
            additional_params: HashMap::new(),
        },
    );

    Config {
        default_provider: provider.to_string(),
        providers,
        use_gitmoji: false,
        custom_instructions: "".to_string(),
    }
}

#[tokio::test]
async fn test_get_refined_message_openai() -> Result<()> {
    setup_mock_providers();
    let git_info = create_mock_git_info();
    let config = create_default_config("openai");

    let result = get_refined_message(&git_info, &config, "openai", false, false, &[], None).await?;

    assert!(result.contains("Mock OpenAI response for:"));
    assert!(result.contains("Branch: main"));

    teardown_mock_providers();
    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_claude() -> Result<()> {
    setup_mock_providers();
    let git_info = create_mock_git_info();
    let config = create_default_config("claude");

    let result = get_refined_message(&git_info, &config, "claude", false, false, &[], None).await?;

    assert!(result.contains("Mock Claude response for:"));
    assert!(result.contains("Branch: main"));

    teardown_mock_providers();
    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_unsupported_provider() {
    setup_mock_providers();
    let git_info = create_mock_git_info();
    let config = create_default_config("openai");

    let result = get_refined_message(
        &git_info,
        &config,
        "unsupported_provider",
        false,
        false,
        &[],
        None,
    )
    .await;

    assert!(result.is_err());

    teardown_mock_providers();
}

#[tokio::test]
async fn test_get_refined_message_with_inpaint_context() -> Result<()> {
    setup_mock_providers();
    let git_info = create_mock_git_info();
    let config = create_default_config("openai");

    let inpaint_context = vec![
        "This commit fixes a critical bug".to_string(),
        "The bug was causing performance issues".to_string(),
    ];

    let result = get_refined_message(
        &git_info,
        &config,
        "openai",
        false,
        false,
        &inpaint_context,
        None,
    )
    .await?;

    assert!(result.contains("Mock OpenAI response for:"));
    assert!(result.contains("Additional context provided by the user:"));
    assert!(result.contains("This commit fixes a critical bug"));
    assert!(result.contains("The bug was causing performance issues"));

    teardown_mock_providers();
    Ok(())
}

#[tokio::test]
async fn test_get_refined_message_with_custom_instructions() -> Result<()> {
    setup_mock_providers();
    let git_info = create_mock_git_info();
    let mut config = create_default_config("openai");
    config.custom_instructions = "Always use imperative mood".to_string();

    let result = get_refined_message(&git_info, &config, "openai", false, false, &[], None).await?;

    assert!(result.contains("Mock OpenAI response for:"));
    assert!(result.contains("Always use imperative mood"));

    teardown_mock_providers();
    Ok(())
}
