use crate::changelog_prompts;
use crate::changelog_prompts::create_release_notes_user_prompt;
use crate::config::Config;
use crate::git;
use crate::llm;
use crate::llm_providers::LLMProviderType;
use crate::readme_reader::{find_and_read_readme, summarize_readme};
use anyhow::{Context, Result};
use std::path::Path;

pub struct ChangelogGenerator;

impl ChangelogGenerator {
    pub async fn generate(
        repo_path: &Path,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let analyzed_changes = git::get_commits_between(repo_path, from, to)?;

        // Find and summarize README
        let readme_content = find_and_read_readme(repo_path)?;
        let readme_summary = if let Some(content) = readme_content {
            let provider_type: LLMProviderType = config.default_provider.parse()?;
            Some(summarize_readme(config, &provider_type, &content).await?)
        } else {
            None
        };

        let mut system_prompt = changelog_prompts::create_changelog_system_prompt(config);
        let effective_instructions = config.get_effective_instructions();
        if !effective_instructions.is_empty() {
            system_prompt.push_str(&format!(
                "\n\nAdditional instructions:\n{}",
                effective_instructions
            ));
        }

        let user_prompt = changelog_prompts::create_changelog_user_prompt(
            &analyzed_changes,
            detail_level,
            from,
            to,
            readme_summary.as_deref(),
        );

        let provider_type: LLMProviderType = config
            .default_provider
            .parse()
            .context("Failed to parse default provider")?;

        let changelog =
            llm::get_refined_message(config, &provider_type, &system_prompt, &user_prompt, None)
                .await
                .context("Failed to generate changelog")?;

        Ok(changelog)
    }
}

pub struct ReleaseNotesGenerator;

impl ReleaseNotesGenerator {
    pub async fn generate(
        repo_path: &Path,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let changelog =
            ChangelogGenerator::generate(repo_path, from, to, config, detail_level).await?;

        // Find and summarize README
        let readme_content = find_and_read_readme(repo_path)?;
        let readme_summary = if let Some(content) = readme_content {
            let provider_type: LLMProviderType = config.default_provider.parse()?;
            Some(summarize_readme(config, &provider_type, &content).await?)
        } else {
            None
        };

        let mut system_prompt = changelog_prompts::create_release_notes_system_prompt(config);
        let effective_instructions = config.get_effective_instructions();
        if !effective_instructions.is_empty() {
            system_prompt.push_str(&format!(
                "\n\nAdditional instructions:\n{}",
                effective_instructions
            ));
        }

        let user_prompt = create_release_notes_user_prompt(
            &changelog,
            detail_level,
            from,
            to,
            readme_summary.as_deref(),
        );

        let provider_type: LLMProviderType = config
            .default_provider
            .parse()
            .context("Failed to parse default provider")?;

        let release_notes =
            llm::get_refined_message(config, &provider_type, &system_prompt, &user_prompt, None)
                .await
                .context("Failed to generate release notes summary")?;

        Ok(release_notes)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DetailLevel {
    Minimal,
    Standard,
    Detailed,
}

impl DetailLevel {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(DetailLevel::Minimal),
            "standard" => Ok(DetailLevel::Standard),
            "detailed" => Ok(DetailLevel::Detailed),
            _ => Err(anyhow::anyhow!("Invalid detail level: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use git2::Repository;
    use tempfile::TempDir;

    fn setup_test_repo() -> Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        let signature = git2::Signature::now("Test User", "test@example.com")?;

        // Create initial commit
        {
            let mut index = repo.index()?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )?;
        }

        // Create a tag for the initial commit
        {
            let head = repo.head()?.peel_to_commit()?;
            repo.tag(
                "v1.0.0",
                &head.into_object(),
                &signature,
                "Version 1.0.0",
                false,
            )?;
        }

        // Create a new file and commit
        std::fs::write(temp_dir.path().join("file1.txt"), "Hello, world!")?;
        {
            let mut index = repo.index()?;
            index.add_path(Path::new("file1.txt"))?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let parent = repo.head()?.peel_to_commit()?;
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Add file1.txt",
                &tree,
                &[&parent],
            )?;
        }

        // Create another tag
        {
            let head = repo.head()?.peel_to_commit()?;
            repo.tag(
                "v1.1.0",
                &head.into_object(),
                &signature,
                "Version 1.1.0",
                false,
            )?;
        }

        Ok((temp_dir, repo))
    }

    #[tokio::test]
    async fn test_changelog_generation() -> Result<()> {
        let (temp_dir, _repo) = setup_test_repo()?;
        let mut config = Config::default();
        config.default_provider = "test".to_string();

        let changelog = ChangelogGenerator::generate(
            temp_dir.path(),
            "v1.0.0",
            "v1.1.0",
            &config,
            DetailLevel::Standard,
        )
        .await?;

        assert!(changelog.contains("Test response from model 'test-model'"));
        assert!(changelog.contains("System prompt:"));
        assert!(changelog.contains("User prompt:"));
        assert!(changelog.contains("v1.0.0"));
        assert!(changelog.contains("v1.1.0"));
        assert!(changelog.contains("Add file1.txt"));

        Ok(())
    }

    #[tokio::test]
    async fn test_release_notes_generation() -> Result<()> {
        let (temp_dir, _repo) = setup_test_repo()?;
        let mut config = Config::default();
        config.default_provider = "test".to_string();

        let release_notes = ReleaseNotesGenerator::generate(
            temp_dir.path(),
            "v1.0.0",
            "v1.1.0",
            &config,
            DetailLevel::Standard,
        )
        .await?;

        assert!(release_notes.contains("Test response from model 'test-model'"));
        assert!(release_notes.contains("System prompt:"));
        assert!(release_notes.contains("User prompt:"));
        assert!(release_notes.contains("v1.0.0"));
        assert!(release_notes.contains("v1.1.0"));
        assert!(release_notes.contains("Add file1.txt"));

        Ok(())
    }

    #[test]
    fn test_detail_level_from_str() {
        assert_eq!(
            DetailLevel::from_str("minimal").unwrap(),
            DetailLevel::Minimal
        );
        assert_eq!(
            DetailLevel::from_str("standard").unwrap(),
            DetailLevel::Standard
        );
        assert_eq!(
            DetailLevel::from_str("detailed").unwrap(),
            DetailLevel::Detailed
        );
        assert!(DetailLevel::from_str("invalid").is_err());
    }
}
