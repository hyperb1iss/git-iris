use anyhow::Result;
use git2::{Repository, Signature};
use git_iris::changelog::{ChangelogGenerator, ReleaseNotesGenerator};
use git_iris::config::Config;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn setup_test_repo() -> Result<(TempDir, Repository)> {
    let temp_dir = TempDir::new()?;
    let repo = Repository::init(temp_dir.path())?;

    let signature = Signature::now("Test User", "test@example.com")?;

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
    fs::write(temp_dir.path().join("file1.txt"), "Hello, world!")?;
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

    let changelog =
        ChangelogGenerator::generate(temp_dir.path(), "v1.0.0", "v1.1.0", &config).await?;
    println!("changelog: {}", changelog);
    assert!(
        changelog.contains("Add file1.txt"),
        "Changelog should mention the new file"
    );
    assert!(
        changelog.contains("v1.0.0"),
        "Changelog should mention the starting tag"
    );
    assert!(
        changelog.contains("v1.1.0"),
        "Changelog should mention the ending tag"
    );

    Ok(())
}

#[tokio::test]
async fn test_release_notes_generation() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let mut config = Config::default();
    config.default_provider = "test".to_string();

    let release_notes =
        ReleaseNotesGenerator::generate(temp_dir.path(), "v1.0.0", "v1.1.0", &config).await?;
    println!("release_notes: {}", release_notes);
    assert!(
        release_notes.contains("Release Notes"),
        "Release notes should have a title"
    );
    assert!(
        release_notes.contains("Add file1.txt"),
        "Release notes should mention the new file"
    );
    assert!(
        release_notes.contains("v1.0.0"),
        "Release notes should mention the starting tag"
    );
    assert!(
        release_notes.contains("v1.1.0"),
        "Release notes should mention the ending tag"
    );
    assert!(
        release_notes.contains("Breaking Changes"),
        "Release notes should have a breaking changes section"
    );

    Ok(())
}

#[tokio::test]
async fn test_changelog_with_head() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let mut config = Config::default();
    config.default_provider = "test".to_string();

    let changelog =
        ChangelogGenerator::generate(temp_dir.path(), "v1.0.0", "HEAD", &config).await?;
    println!("changelog: {}", changelog);
    assert!(
        changelog.contains("Add file1.txt"),
        "Changelog should mention the new file"
    );
    assert!(
        changelog.contains("v1.0.0"),
        "Changelog should mention the starting tag"
    );
    assert!(
        changelog.contains("HEAD") || changelog.contains("v1.1.0"),
        "Changelog should mention HEAD or the latest tag"
    );

    Ok(())
}
