#![allow(clippy::unwrap_used)]
#![allow(clippy::no_effect_underscore_binding)]

use git_iris::{
    agents::{
        core::{AgentBackend, AgentContext, TaskResult},
        iris::IrisAgentBuilder,
        setup::{AgentSetupService, create_agent_with_defaults},
        tools::{
            GitBlame, GitChangedFiles, GitDiff, GitLog, GitRepoInfo, GitStatus, ProjectDocs,
            current_repo_root, get_current_repo, with_active_repo_root,
        },
    },
    config::Config,
    git::GitRepo,
};
use rig::tool::portable::PortableTool;
use std::env;
use std::fs;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

use git_iris::agents::tools::docs::{DocType, ProjectDocsArgs};

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_git_repo;

fn cwd_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
}

fn create_test_context() -> (AgentContext, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize a git repo in the temp directory
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to initialize git repo");

    let config = Config::default();
    let git_repo = GitRepo::new(&repo_path).expect("Failed to create GitRepo");

    let context = AgentContext::new(config, git_repo);
    (context, temp_dir)
}

#[test]
fn test_agent_backend_creation() {
    let backend = AgentBackend::new(
        "openai".to_string(),
        "gpt-5.4".to_string(),
        "gpt-5.4-mini".to_string(),
    );
    assert_eq!(backend.provider_name, "openai");
    assert_eq!(backend.model, "gpt-5.4");
    assert_eq!(backend.fast_model, "gpt-5.4-mini");
}

#[test]
fn test_agent_backend_from_config() {
    let config = Config::default();
    let backend = AgentBackend::from_config(&config);
    assert!(backend.is_ok());

    let backend = backend.unwrap();
    assert!(!backend.provider_name.is_empty());
    assert!(!backend.model.is_empty());
    assert!(!backend.fast_model.is_empty());
}

#[test]
fn test_agent_context_creation() {
    let (context, _temp_dir) = create_test_context();

    assert!(!context.config().default_provider.is_empty());
    assert!(context.repo().repo_path().exists());
}

#[test]
fn test_task_result_creation() {
    let result = TaskResult::success("Test completed".to_string());
    assert!(result.success);
    assert_eq!(result.message, "Test completed");
    assert!((result.confidence - 1.0).abs() < f64::EPSILON);

    let failure = TaskResult::failure("Test failed".to_string());
    assert!(!failure.success);
    assert_eq!(failure.message, "Test failed");
    assert!((failure.confidence - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_task_result_with_data() {
    let data = serde_json::json!({"test": "value"});
    let result = TaskResult::success_with_data("Test with data".to_string(), data.clone());

    assert!(result.success);
    assert_eq!(result.data, Some(data));
    assert!((result.confidence - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_task_result_with_confidence() {
    let result = TaskResult::success("Test".to_string()).with_confidence(0.8);
    assert!((result.confidence - 0.8).abs() < f64::EPSILON);
}

#[test]
fn test_task_result_with_execution_time() {
    let duration = std::time::Duration::from_millis(500);
    let result = TaskResult::success("Test".to_string()).with_execution_time(duration);
    assert_eq!(result.execution_time, Some(duration));
}

#[test]
fn test_iris_agent_builder() {
    // Builder now creates client on-demand from environment
    let result = IrisAgentBuilder::new()
        .with_provider("openai")
        .with_model("gpt-5.4")
        .with_preamble("Custom preamble")
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_iris_agent_builder_defaults() {
    // Builder with defaults (openai/gpt-5.4) should succeed
    let result = IrisAgentBuilder::new().build();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_agent_with_defaults() {
    // Skip if no API key is available
    if env::var("OPENAI_API_KEY").is_err() && env::var("ANTHROPIC_API_KEY").is_err() {
        return;
    }

    let result = create_agent_with_defaults("openai", "gpt-5.4");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_setup_service_creation() {
    let config = Config::default();
    let setup_service = AgentSetupService::new(config);

    assert!(!setup_service.config().default_provider.is_empty());
    assert!(setup_service.git_repo().is_none()); // No repo set initially
}

#[tokio::test]
async fn test_agent_setup_service_from_temp_dir() {
    let _guard = cwd_lock();
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize a git repo in the temp directory
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to initialize git repo");

    // Change to the temp directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&repo_path).unwrap();

    // Run the test - already in a tokio runtime, no need for nested runtime
    let common_params = git_iris::common::CommonParams::default();
    let setup_service = AgentSetupService::from_common_params(&common_params, None);

    // Restore original directory before assertions
    env::set_current_dir(original_dir).unwrap();

    assert!(setup_service.is_ok());
    let setup_service = setup_service.unwrap();
    assert!(setup_service.git_repo().is_some());
}

#[test]
fn test_git_tools_exist() {
    // Test that our Git tools are available and have proper types
    let _git_status = GitStatus;
    let _git_diff = GitDiff;
    let _git_log = GitLog;
    let _git_blame = GitBlame;
    let _git_repo_info = GitRepoInfo;
    let _git_changed_files = GitChangedFiles;

    // If we get here, all tools compiled successfully
}

#[test]
fn test_agent_context_accessors() {
    let (context, _temp_dir) = create_test_context();

    // Test that we can access the config and repo
    let config = context.config();
    let repo = context.repo();

    assert!(!config.default_provider.is_empty());
    assert!(repo.repo_path().exists());
}

#[test]
fn test_git_repo_discovers_root_from_subdirectory() {
    let (temp_dir, _git_repo) = setup_git_repo();
    let nested_dir = temp_dir.path().join("nested").join("path");
    fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");

    let repo = GitRepo::new(&nested_dir).expect("Failed to create GitRepo from subdirectory");
    assert_eq!(
        fs::canonicalize(repo.repo_path()).expect("Failed to canonicalize repo path"),
        fs::canonicalize(temp_dir.path()).expect("Failed to canonicalize temp repo path")
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_tool_repo_context_uses_active_repo_root() {
    let _guard = cwd_lock();
    let (primary_temp_dir, _primary_repo) = setup_git_repo();
    let (fallback_temp_dir, _fallback_repo) = setup_git_repo();
    let original_dir = env::current_dir().expect("Failed to get current directory");

    fs::write(
        primary_temp_dir.path().join("README.md"),
        "# Primary Repo\n",
    )
    .expect("Failed to update primary README");
    fs::write(
        fallback_temp_dir.path().join("README.md"),
        "# Fallback Repo\n",
    )
    .expect("Failed to update fallback README");

    env::set_current_dir(fallback_temp_dir.path()).expect("Failed to change current directory");

    let repo = with_active_repo_root(primary_temp_dir.path(), async {
        get_current_repo().expect("Failed to resolve active repository")
    })
    .await;
    assert_eq!(
        fs::canonicalize(repo.repo_path()).expect("Failed to canonicalize active repo path"),
        fs::canonicalize(primary_temp_dir.path())
            .expect("Failed to canonicalize primary repo path")
    );

    let repo_root = with_active_repo_root(primary_temp_dir.path(), async {
        current_repo_root().expect("Failed to resolve active repo root")
    })
    .await;
    assert_eq!(
        fs::canonicalize(repo_root).expect("Failed to canonicalize resolved repo root"),
        fs::canonicalize(primary_temp_dir.path())
            .expect("Failed to canonicalize primary repo root")
    );

    let docs = with_active_repo_root(primary_temp_dir.path(), async {
        ProjectDocs
            .call(ProjectDocsArgs {
                doc_type: DocType::Readme,
                max_chars: 2_000,
            })
            .await
            .expect("Failed to read project docs")
    })
    .await;

    env::set_current_dir(original_dir).expect("Failed to restore current directory");

    assert!(docs.contains("Primary Repo"));
    assert!(!docs.contains("Fallback Repo"));
}

// Integration test for the complete agent setup workflow
#[tokio::test]
async fn test_complete_agent_setup_workflow() {
    // Skip if no API key is available
    if env::var("OPENAI_API_KEY").is_err() && env::var("ANTHROPIC_API_KEY").is_err() {
        return;
    }

    let _guard = cwd_lock();
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize a git repo in the temp directory
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to initialize git repo");

    // Test the complete workflow
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&repo_path).unwrap();

    // Already in a tokio runtime, no need for nested runtime
    let common_params = git_iris::common::CommonParams::default();
    let setup_service = AgentSetupService::from_common_params(&common_params, None);

    // This will fail without proper API keys, but tests the pipeline
    if let Ok(mut setup_service) = setup_service {
        let agent_result = setup_service.create_iris_agent();

        // We expect this to fail in CI/testing without API keys
        // but the error should be about missing API keys, not code structure
        if let Err(e) = agent_result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("API key")
                    || error_msg.contains("OPENAI_API_KEY")
                    || error_msg.contains("ANTHROPIC_API_KEY")
                    || error_msg.contains("configuration")
                    || error_msg.contains("provider"),
                "Expected API key or configuration error, got: {error_msg}"
            );
        }
    }

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
}
