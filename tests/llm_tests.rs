#[cfg(test)]
mod tests {
    use anyhow::Result;
    use async_trait::async_trait;
    use git_iris::config::Config;
    use git_iris::git::{FileChange, GitInfo};
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

        let git_info = create_mock_git_info();
        let config = Config::default();

        let result =
            get_refined_message(&git_info, &config, "openai", false, false, None, "").await?;
        assert_eq!(result, "Mocked commit message");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_refined_message_claude() -> Result<()> {
        init_mock_providers();

        let git_info = create_mock_git_info();
        let config = Config::default();

        let result =
            get_refined_message(&git_info, &config, "claude", false, false, None, "").await?;
        assert_eq!(result, "Mocked commit message");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_refined_message_unsupported_provider() -> Result<()> {
        let git_info = create_mock_git_info();
        let config = Config::default();

        let result = get_refined_message(
            &git_info,
            &config,
            "unsupported_provider",
            false,
            false,
            None,
            "",
        )
        .await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_refined_message_with_existing_message() -> Result<()> {
        init_mock_providers();

        let git_info = create_mock_git_info();
        let config = Config::default();
        let existing_message = "Initial commit message";

        let result = get_refined_message(
            &git_info,
            &config,
            "openai",
            false,
            false,
            Some(existing_message),
            "",
        )
        .await?;

        assert_eq!(result, "Mocked commit message");
        Ok(())
    }
}
