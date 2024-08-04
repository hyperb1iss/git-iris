use anyhow::Result;
use async_trait::async_trait;
use git_iris::config::{Config, ProviderConfig};
use git_iris::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use git_iris::llm::{
    get_available_provider_names, get_default_model_for_provider,
    get_default_token_limit_for_provider, get_refined_message_with_provider,
};
use git_iris::llm_providers::{LLMProvider, LLMProviderConfig, LLMProviderType};
use mockall::mock;
use mockall::predicate::*;
use std::collections::HashMap;

// Mock the LLMProvider trait
mock! {
    pub LLMProvider {}
    #[async_trait]
    impl LLMProvider for LLMProvider {
        async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
    }
}

// Helper function to create a mock commit context
fn create_mock_commit_context() -> CommitContext {
    CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![RecentCommit {
            hash: "abcdef1".to_string(),
            message: "Initial commit".to_string(),
            author: "Test User".to_string(),
            timestamp: "1234567890".to_string(),
        }],
        staged_files: vec![StagedFile {
            path: "file1.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "- old line\n+ new line".to_string(),
            analysis: vec!["Modified function: main".to_string()],
            content_excluded: false,
        }],
        unstaged_files: vec!["unstaged_file.txt".to_string()],
        project_metadata: ProjectMetadata {
            language: Some("Rust".to_string()),
            framework: None,
            dependencies: vec![],
            version: None,
            build_system: None,
            test_framework: None,
            plugins: vec![],
        },
    }
}

// Helper function to create a mock configuration
fn create_mock_config() -> Config {
    let mut config = Config::default();
    config.providers.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: "mock_openai_api_key".to_string(),
            model: "gpt-4o".to_string(),
            additional_params: HashMap::new(),
            token_limit: Some(8000),
        },
    );
    config
}

#[tokio::test]
async fn test_get_refined_message() -> Result<()> {
    let commit_context = create_mock_commit_context();
    let config = create_mock_config();

    // Create a closure that returns a new mock provider each time
    let create_provider = |_provider_type: LLMProviderType, _: LLMProviderConfig| {
        let mut mock_provider = MockLLMProvider::new();
        mock_provider
            .expect_generate_message()
            .returning(|_, _| Ok("Mocked commit message".to_string()));
        Ok(Box::new(mock_provider) as Box<dyn LLMProvider>)
    };

    // Call get_refined_message_with_provider with the mock provider
    let result = get_refined_message_with_provider(
        &commit_context,
        &config,
        "openai",
        false,
        false,
        "",
        create_provider,
    )
    .await?;
    assert_eq!(result, "Mocked commit message");
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
    assert_eq!(get_default_model_for_provider("openai")?, "gpt-4o");
    assert_eq!(
        get_default_model_for_provider("claude")?,
        "claude-3-5-sonnet-20240620"
    );
    Ok(())
}

#[test]
fn test_get_default_token_limit_for_provider() -> Result<()> {
    assert_eq!(get_default_token_limit_for_provider("openai")?, 100000);
    assert_eq!(get_default_token_limit_for_provider("claude")?, 150000);
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
