//! Common utilities for agent tools
//!
//! This module provides shared functionality used across multiple tools:
//! - Schema generation for OpenAI-compatible tool definitions
//! - Error type macros
//! - Repository initialization helpers

use std::future::Future;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::git::GitRepo;

tokio::task_local! {
    static ACTIVE_REPO_ROOT: PathBuf;
    static TRUSTED_EXECUTION: bool;
}

/// Generate a JSON schema for tool parameters that's `OpenAI`-compatible.
/// `OpenAI` tool schemas require the `required` array to list every property.
///
/// # Panics
///
/// Panics if the generated schema cannot be serialized to JSON.
#[must_use]
pub fn parameters_schema<T: schemars::JsonSchema>() -> Value {
    use schemars::schema_for;

    let schema = schema_for!(T);
    let mut value = serde_json::to_value(schema).expect("tool schema should serialize");
    enforce_required_properties(&mut value);
    value
}

/// Ensure all properties are listed in the `required` array.
/// This is needed for `OpenAI` tool compatibility.
fn enforce_required_properties(value: &mut Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };

    let props_entry = obj
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()));
    let props_obj = props_entry.as_object().expect("properties must be object");
    let required_keys: Vec<Value> = props_obj.keys().cloned().map(Value::String).collect();

    obj.insert("required".to_string(), Value::Array(required_keys));
}

/// Get the current repository from the working directory.
/// This is a common operation used by most tools.
///
/// # Errors
///
/// Returns an error when the active repository root cannot be resolved.
pub fn get_current_repo() -> anyhow::Result<GitRepo> {
    let repo_root = current_repo_root()?;
    GitRepo::new(&repo_root)
}

/// Run an async operation with a repo root bound to the current task.
pub async fn with_active_repo_root<F, T>(repo_path: &Path, future: F) -> T
where
    F: Future<Output = T>,
{
    with_repo_execution_context(repo_path, current_repo_execution_trusted(), future).await
}

/// Bind repository identity and permission to execute project code to a task.
pub async fn with_repo_execution_context<F, T>(repo_path: &Path, trusted: bool, future: F) -> T
where
    F: Future<Output = T>,
{
    ACTIVE_REPO_ROOT
        .scope(
            repo_path.to_path_buf(),
            TRUSTED_EXECUTION.scope(trusted, future),
        )
        .await
}

/// Local workspaces permit analysis; remote clones must explicitly carry distrust.
#[must_use]
pub fn current_repo_execution_trusted() -> bool {
    TRUSTED_EXECUTION
        .try_with(|trusted| *trusted)
        .unwrap_or(true)
}

/// Get the repo root bound to the current task, falling back to the current directory.
///
/// # Errors
///
/// Returns an error when the current directory cannot be resolved to a Git repository.
pub fn current_repo_root() -> anyhow::Result<PathBuf> {
    if let Ok(repo_root) = ACTIVE_REPO_ROOT.try_with(Clone::clone) {
        return Ok(repo_root);
    }

    let current_dir = std::env::current_dir()?;
    let repo = GitRepo::new(&current_dir)?;
    Ok(repo.repo_path().clone())
}

/// Macro to define a tool error type with standard From implementations.
///
/// This creates a newtype wrapper around String that implements:
/// - `Debug`, `thiserror::Error`
/// - `From<anyhow::Error>`
/// - `From<std::io::Error>`
///
/// # Example
/// ```ignore
/// define_tool_error!(GitError);
/// // Creates: pub struct GitError(String);
/// // With Display showing: "GitError: {message}"
/// ```
#[macro_export]
macro_rules! define_tool_error {
    ($name:ident) => {
        #[derive(Debug)]
        pub struct $name(pub String);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::error::Error for $name {}

        impl From<anyhow::Error> for $name {
            fn from(err: anyhow::Error) -> Self {
                $name(err.to_string())
            }
        }

        impl From<std::io::Error> for $name {
            fn from(err: std::io::Error) -> Self {
                $name(err.to_string())
            }
        }
    };
}

// Re-export the macro at the module level
pub use define_tool_error;
