#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]

use git_iris::config::Config;
use git_iris::context::{ChangeType, CommitContext, RecentCommit, StagedFile};
use git_iris::git::GitRepo;
use git_iris::providers::ProviderConfig;
use git_iris::types::{ChangeMetrics, MarkdownPullRequest};
use git2::{BranchType, Repository};

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Creates a temporary Git repository with an initial commit for testing
#[allow(dead_code)]
pub fn setup_git_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo = Repository::init(temp_dir.path()).expect("Failed to initialize repository");

    // Configure git user
    let mut config = repo.config().expect("Failed to get repository config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user email");

    // Create and commit an initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Initial content").expect("Failed to write initial file");

    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("initial.txt"))
        .expect("Failed to add file to index");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let signature = repo.signature().expect("Failed to create signature");
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .expect("Failed to commit");

    // Normalize the primary branch to `main` so tests do not depend on the
    // Git version or runner-wide init.defaultBranch configuration.
    ensure_primary_branch(&repo, "main");

    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    (temp_dir, git_repo)
}

fn ensure_primary_branch(repo: &Repository, target_branch: &str) {
    let current_branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().ok().map(std::string::ToString::to_string))
        .unwrap_or_default();
    if current_branch == target_branch {
        return;
    }

    let head_commit = repo
        .head()
        .expect("Failed to get HEAD")
        .peel_to_commit()
        .expect("Failed to peel HEAD to commit");

    repo.branch(target_branch, &head_commit, true)
        .expect("Failed to create target branch");
    repo.set_head(&format!("refs/heads/{target_branch}"))
        .expect("Failed to set HEAD to target branch");
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
        .expect("Failed to checkout target branch");

    if !current_branch.is_empty()
        && current_branch != target_branch
        && let Ok(mut branch) = repo.find_branch(&current_branch, BranchType::Local)
    {
        branch
            .delete()
            .expect("Failed to delete original primary branch");
    }
}

/// Creates a Git repository with tags for changelog/release notes testing
#[allow(dead_code)]
pub fn setup_git_repo_with_tags() -> Result<(TempDir, Repository)> {
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

/// Creates a Git repository with multiple commits for PR testing
#[allow(dead_code)]
pub fn setup_git_repo_with_commits() -> Result<(TempDir, GitRepo)> {
    let temp_dir = TempDir::new()?;
    let repo = Repository::init(temp_dir.path())?;

    // Configure git user
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;

    // Create initial commit
    let signature = git2::Signature::now("Test User", "test@example.com")?;

    // Create initial file
    fs::write(temp_dir.path().join("README.md"), "# Initial Project")?;
    let mut index = repo.index()?;
    index.add_path(Path::new("README.md"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let initial_commit = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )?;

    // Create src directory and second commit
    fs::create_dir_all(temp_dir.path().join("src"))?;
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )?;
    index.add_path(Path::new("src/main.rs"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent_commit = repo.find_commit(initial_commit)?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Add main function",
        &tree,
        &[&parent_commit],
    )?;

    let git_repo = GitRepo::new(temp_dir.path())?;
    Ok((temp_dir, git_repo))
}

/// Creates a minimal temporary directory with just a `GitRepo` (no git initialization)
#[allow(dead_code)]
pub fn setup_temp_dir() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    (temp_dir, git_repo)
}

/// Git repository operations helper
#[allow(dead_code)]
pub struct GitTestHelper<'a> {
    pub temp_dir: &'a TempDir,
    pub repo: Repository,
}

#[allow(dead_code)]
impl<'a> GitTestHelper<'a> {
    pub fn new(temp_dir: &'a TempDir) -> Result<Self> {
        let repo = Repository::open(temp_dir.path())?;
        Ok(Self { temp_dir, repo })
    }

    /// Create and stage a file
    pub fn create_and_stage_file(&self, path: &str, content: &str) -> Result<()> {
        let file_path = self.temp_dir.path().join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;

        let mut index = self.repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    /// Create a commit with the staged files
    pub fn commit(&self, message: &str) -> Result<git2::Oid> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let signature = self.repo.signature()?;

        let parent_commit = if let Ok(head) = self.repo.head() {
            Some(head.peel_to_commit()?)
        } else {
            None
        };

        let parents: Vec<&git2::Commit> = parent_commit.as_ref().into_iter().collect();

        Ok(self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) -> Result<()> {
        let head_commit = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head_commit, false)?;
        Ok(())
    }

    /// Switch to a branch
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        let branch = self.repo.find_branch(name, git2::BranchType::Local)?;
        let branch_name = branch
            .get()
            .name()
            .expect("Branch should have a valid name");
        self.repo.set_head(branch_name)?;
        self.repo
            .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    }

    /// Create a tag
    pub fn create_tag(&self, name: &str, message: &str) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;
        let signature = self.repo.signature()?;
        self.repo
            .tag(name, &head.into_object(), &signature, message, false)?;
        Ok(())
    }
}

// Mock data creators
#[allow(dead_code)]
pub struct MockDataBuilder;

#[allow(dead_code)]
impl MockDataBuilder {
    /// Create a mock `CommitContext` for testing
    pub fn commit_context() -> CommitContext {
        CommitContext {
            branch: "main".to_string(),
            recent_commits: vec![RecentCommit {
                hash: "abcdef1".to_string(),
                message: "Initial commit".to_string(),
                author: "Test User".to_string(),
                timestamp: "1234567890".to_string(),
            }],
            staged_files: vec![Self::staged_file()],
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
        }
    }

    /// Create a mock `CommitContext` for PR testing
    pub fn pr_commit_context() -> CommitContext {
        CommitContext {
            branch: "main..feature-auth".to_string(),
            recent_commits: vec![
                RecentCommit {
                    hash: "abc1234".to_string(),
                    message: "Add JWT authentication middleware".to_string(),
                    author: "Test User".to_string(),
                    timestamp: "1234567890".to_string(),
                },
                RecentCommit {
                    hash: "def5678".to_string(),
                    message: "Implement user registration endpoint".to_string(),
                    author: "Test User".to_string(),
                    timestamp: "1234567891".to_string(),
                },
            ],
            staged_files: vec![
                StagedFile {
                    path: "src/auth/middleware.rs".to_string(),
                    change_type: ChangeType::Added,
                    diff: "+ use jwt::encode;\n+ pub fn auth_middleware() -> impl Filter<Extract = (), Error = Rejection> + Clone {".to_string(),
                    content: Some("use jwt::encode;\n\npub fn auth_middleware() -> impl Filter {}".to_string()),
                    content_excluded: false,
                },
                StagedFile {
                    path: "src/auth/models.rs".to_string(),
                    change_type: ChangeType::Added,
                    diff: "+ #[derive(Serialize, Deserialize)]\n+ pub struct User {".to_string(),
                    content: Some("#[derive(Serialize, Deserialize)]\npub struct User {\n    pub id: u32,\n    pub email: String,\n}".to_string()),
                    content_excluded: false,
                },
            ],
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
        }
    }

    /// Create a mock `StagedFile`
    pub fn staged_file() -> StagedFile {
        StagedFile {
            path: "file1.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "- old line\n+ new line".to_string(),
            content: None,
            content_excluded: false,
        }
    }

    /// Create a mock `StagedFile` with specific properties
    pub fn staged_file_with(path: &str, change_type: ChangeType, diff: &str) -> StagedFile {
        StagedFile {
            path: path.to_string(),
            change_type,
            diff: diff.to_string(),
            content: None,
            content_excluded: false,
        }
    }

    /// Create a mock Config
    pub fn config() -> Config {
        Config::default()
    }

    /// Create a mock Config with gitmoji enabled
    pub fn config_with_gitmoji() -> Config {
        Config {
            use_gitmoji: true,
            ..Default::default()
        }
    }

    /// Create a mock Config with custom instructions
    pub fn config_with_instructions(instructions: &str) -> Config {
        Config {
            instructions: instructions.to_string(),
            ..Default::default()
        }
    }

    /// Create a mock test Config with API key
    pub fn test_config_with_api_key(provider: &str, api_key: &str) -> Config {
        let provider_config = ProviderConfig {
            api_key: api_key.to_string(),
            model: "test-model".to_string(),
            ..Default::default()
        };

        Config {
            default_provider: provider.to_string(),
            providers: [(provider.to_string(), provider_config)]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    /// Create mock `ChangeMetrics`
    pub fn change_metrics() -> ChangeMetrics {
        ChangeMetrics {
            total_commits: 1,
            files_changed: 1,
            insertions: 15,
            deletions: 5,
            total_lines_changed: 20,
        }
    }

    /// Create mock total `ChangeMetrics`
    pub fn total_change_metrics() -> ChangeMetrics {
        ChangeMetrics {
            total_commits: 5,
            files_changed: 10,
            insertions: 100,
            deletions: 50,
            total_lines_changed: 150,
        }
    }

    /// Create a mock `MarkdownPullRequest`
    pub fn generated_pull_request() -> MarkdownPullRequest {
        MarkdownPullRequest {
            content: r"# Add JWT authentication with user registration

## Summary

Implements comprehensive JWT-based authentication system with user registration, login, and middleware for protected routes.

## Description

This PR introduces a complete authentication system:

**Features Added:**
- JWT token generation and validation
- User registration endpoint
- Authentication middleware for protected routes
- Password hashing with bcrypt

**Technical Details:**
- Uses industry-standard JWT libraries
- Implements secure password storage
- Includes comprehensive error handling

## Commits

- `abc1234`: Add JWT authentication middleware
- `def5678`: Implement user registration endpoint

## Breaking Changes

- All protected endpoints now require authentication headers

## Testing

Test user registration flow and verify JWT tokens are properly validated on protected routes.

## Notes

Requires JWT_SECRET environment variable to be set before deployment.
".to_string(),
        }
    }

    /// Create a mock binary file for testing
    pub fn mock_binary_content() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ]
    }
}

/// Test assertion helpers
#[allow(dead_code)]
pub struct TestAssertions;

#[allow(dead_code)]
impl TestAssertions {
    /// Assert that a commit context has expected properties
    pub fn assert_commit_context_basics(context: &CommitContext) {
        assert!(!context.branch.is_empty(), "Branch should not be empty");
        assert!(
            !context.user_name.is_empty(),
            "User name should not be empty"
        );
        assert!(
            !context.user_email.is_empty(),
            "User email should not be empty"
        );
    }

    /// Assert that staged files contain expected changes
    pub fn assert_staged_files_not_empty(context: &CommitContext) {
        assert!(!context.staged_files.is_empty(), "Should have staged files");
    }

    /// Assert that a string contains gitmoji
    pub fn assert_contains_gitmoji(text: &str) {
        let gitmoji_chars = ["✨", "🐛", "📝", "💄", "♻️", "✅", "🔨"];
        assert!(
            gitmoji_chars.iter().any(|&emoji| text.contains(emoji)),
            "Text should contain gitmoji: {text}"
        );
    }

    /// Assert that a prompt contains essential commit information
    pub fn assert_commit_prompt_essentials(prompt: &str) {
        assert!(
            prompt.contains("Branch:"),
            "Prompt should contain branch info"
        );
        assert!(prompt.contains("commit"), "Prompt should mention commits");
    }

    /// Assert that token count is within limit
    pub fn assert_token_limit(actual: usize, limit: usize) {
        assert!(
            actual <= limit,
            "Token count ({actual}) exceeds limit ({limit})"
        );
    }
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Git hooks testing utilities (Unix only)
#[cfg(unix)]
#[allow(dead_code)]
pub struct GitHooksTestHelper;

#[cfg(unix)]
#[allow(dead_code)]
impl GitHooksTestHelper {
    /// Create a git hook script
    pub fn create_hook(
        repo_path: &Path,
        hook_name: &str,
        content: &str,
        should_fail: bool,
    ) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let hooks_dir = repo_path.join(".git").join("hooks");
        fs::create_dir_all(&hooks_dir)?;
        let hook_path = hooks_dir.join(hook_name);
        let mut file = File::create(&hook_path)?;
        writeln!(file, "#!/bin/sh")?;
        writeln!(file, "echo \"Running {hook_name} hook\"")?;
        writeln!(file, "{content}")?;
        if should_fail {
            writeln!(file, "exit 1")?;
        } else {
            writeln!(file, "exit 0")?;
        }
        file.flush()?;

        // Make the hook executable
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;

        Ok(())
    }
}

/// Environment helpers for testing
#[allow(dead_code)]
pub struct TestEnvironment;

#[allow(dead_code)]
impl TestEnvironment {
    /// Check if we should skip remote tests
    pub fn should_skip_remote_tests() -> bool {
        std::env::var("CI").is_ok() || std::env::var("SKIP_REMOTE_TESTS").is_ok()
    }

    /// Check if we should skip integration tests
    pub fn should_skip_integration_tests() -> bool {
        std::env::var("SKIP_INTEGRATION_TESTS").is_ok()
    }

    /// Setup for tests that need API keys
    pub fn setup_api_test_env() -> Option<String> {
        std::env::var("OPENAI_API_KEY").ok()
    }
}
