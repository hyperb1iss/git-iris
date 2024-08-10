use anyhow::Result;
use git_iris::config::Config;
use git_iris::llm_providers::LLMProviderType;
use git_iris::service::GitIrisService;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_test_repo() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let repo = git2::Repository::init(temp_dir.path())?;

    // Create an initial commit
    let signature = git2::Signature::now("Test User", "test@example.com")?;
    let tree_id = repo.index()?.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )?;

    Ok(temp_dir)
}

#[tokio::test]
async fn test_generate_message() -> Result<()> {
    let temp_dir = setup_test_repo()?;
    let config = Config::default();
    let repo_path = PathBuf::from(temp_dir.path());
    let provider_type = LLMProviderType::Test;
    let use_gitmoji = true;

    let service = GitIrisService::new(config, repo_path, provider_type, use_gitmoji);

    let result = service
        .generate_message("default", "Test instructions")
        .await;

    assert!(result.is_ok());
    let message = result.unwrap();
    println!("{:?}", message);
    println!("... message {:?}", message.title);
    println!("... title {:?}", message.message);
    assert!(message.title.contains("<title>"));
    assert!(message.message.contains("<message>"));

    Ok(())
}

#[test]
fn test_perform_commit() -> Result<()> {
    let temp_dir = setup_test_repo()?;
    let config = Config::default();
    let repo_path = PathBuf::from(temp_dir.path());
    let provider_type = LLMProviderType::Test;
    let use_gitmoji = true;

    let service = GitIrisService::new(config, repo_path.clone(), provider_type, use_gitmoji);

    let result = service.perform_commit("Test commit message");

    assert!(result.is_ok());

    // Verify the commit was made
    let repo = git2::Repository::open(&repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(head_commit.message().unwrap(), "Test commit message");

    Ok(())
}

#[test]
fn test_check_environment() -> Result<()> {
    let temp_dir = setup_test_repo()?;
    let config = Config::default();
    let repo_path = PathBuf::from(temp_dir.path());
    let provider_type = LLMProviderType::Test;
    let use_gitmoji = true;

    let service = GitIrisService::new(config, repo_path, provider_type, use_gitmoji);

    let result = service.check_environment();

    assert!(result.is_ok());

    Ok(())
}
