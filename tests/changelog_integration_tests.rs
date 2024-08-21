use anyhow::Result;
use git2::Repository;
use git_iris::changes::{ChangelogGenerator, ReleaseNotesGenerator};
use git_iris::changes::models::{ChangelogResponse, ReleaseNotesResponse};
use git_iris::common::DetailLevel;
use git_iris::config::Config;
use git_iris::llm_providers::LLMProviderType;
use std::env;
use tempfile::TempDir;
use std::path::Path;

// Reuse the setup_test_repo function from changelog_tests.rs
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

    // Create a tag for the initial commit (v1.0.0)
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

    // Create another tag (v1.1.0)
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
#[ignore] // This test requires API keys and will be ignored by default
async fn test_changelog_generation() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let mut config = Config::default();
    config.default_provider = LLMProviderType::OpenAI.to_string(); // Or whichever provider you want to test
    config.providers.get_mut(&config.default_provider).unwrap().api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let changelog = ChangelogGenerator::generate(
        temp_dir.path(),
        "v1.0.0",
        "v1.1.0",
        &config,
        DetailLevel::Standard,
    )
    .await?;

    let changelog_response: ChangelogResponse = serde_json::from_str(&changelog)?;

    assert!(changelog_response.version.is_some());
    assert!(changelog_response.release_date.is_some());
    assert!(!changelog_response.sections.is_empty());
    assert!(changelog_response.metrics.total_commits > 0);
    assert!(changelog_response.metrics.files_changed > 0);

    Ok(())
}

#[tokio::test]
#[ignore] // This test requires API keys and will be ignored by default
async fn test_release_notes_generation() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let mut config = Config::default();
    config.default_provider = LLMProviderType::OpenAI.to_string(); // Or whichever provider you want to test
    config.providers.get_mut(&config.default_provider).unwrap().api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let release_notes = ReleaseNotesGenerator::generate(
        temp_dir.path(),
        "v1.0.0",
        "v1.1.0",
        &config,
        DetailLevel::Standard,
    )
    .await?;

    let release_notes_response: ReleaseNotesResponse = serde_json::from_str(&release_notes)?;

    assert!(release_notes_response.version.is_some());
    assert!(release_notes_response.release_date.is_some());
    assert!(!release_notes_response.summary.is_empty());
    assert!(release_notes_response.metrics.total_commits > 0);
    assert!(release_notes_response.metrics.files_changed > 0);

    Ok(())
}