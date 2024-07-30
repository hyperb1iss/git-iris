#[cfg(test)]
mod tests {
    use anyhow::Result;
    use async_trait::async_trait;
    use git_iris::config::Config;
    use git_iris::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
    use git_iris::llm::{get_refined_message, init_providers, LLMProvider};
    use std::collections::HashMap;
    use std::sync::Arc;

    struct MockLLMProvider;

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn generate_message(
            &self,
            _system_prompt: &str,
            _user_prompt: &str,
        ) -> Result<String> {
            Ok("Mocked commit message".to_string())
        }

        fn is_unsupported(&self) -> bool {
            false
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

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

    fn init_mock_providers() {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            Arc::new(MockLLMProvider) as Arc<dyn LLMProvider>,
        );
        providers.insert(
            "claude".to_string(),
            Arc::new(MockLLMProvider) as Arc<dyn LLMProvider>,
        );
        init_providers(providers);
    }

    #[tokio::test]
    async fn test_get_refined_message_openai() -> Result<()> {
        init_mock_providers();

        let commit_context = create_mock_commit_context();
        let config = Config::default();

        let result =
            get_refined_message(&commit_context, &config, "openai", false, false, "").await?;
        assert_eq!(result, "Mocked commit message");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_refined_message_claude() -> Result<()> {
        init_mock_providers();

        let commit_context = create_mock_commit_context();
        let config = Config::default();

        let result =
            get_refined_message(&commit_context, &config, "claude", false, false, "").await?;
        assert_eq!(result, "Mocked commit message");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_refined_message_unsupported_provider() -> Result<()> {
        let commit_context = create_mock_commit_context();
        let config = Config::default();

        let result = get_refined_message(
            &commit_context,
            &config,
            "unsupported_provider",
            false,
            false,
            "",
        )
        .await;

        assert!(result.is_err());
        Ok(())
    }
}
