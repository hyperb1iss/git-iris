use crate::token_optimizer::TokenOptimizer;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use textwrap::wrap;

#[derive(Serialize, Debug, Clone)]
pub struct CommitContext {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub project_metadata: ProjectMetadata,
    pub user_name: String,
    pub user_email: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct RecentCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct StagedFile {
    pub path: String,
    pub change_type: ChangeType,
    pub diff: String,
    pub analysis: Vec<String>,
    pub content: Option<String>,
    pub content_excluded: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GeneratedMessage {
    pub emoji: Option<String>,
    pub title: String,
    pub message: String,
}

impl From<String> for GeneratedMessage {
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap()
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeType::Added => write!(f, "Added"),
            ChangeType::Modified => write!(f, "Modified"),
            ChangeType::Deleted => write!(f, "Deleted"),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ProjectMetadata {
    pub language: Option<String>,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub version: Option<String>,
    pub build_system: Option<String>,
    pub test_framework: Option<String>,
    pub plugins: Vec<String>,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        ProjectMetadata {
            language: None,
            framework: None,
            dependencies: Vec::new(),
            version: None,
            build_system: None,
            test_framework: None,
            plugins: Vec::new(),
        }
    }
}

impl CommitContext {
    pub fn new(
        branch: String,
        recent_commits: Vec<RecentCommit>,
        staged_files: Vec<StagedFile>,
        project_metadata: ProjectMetadata,
        user_name: String,
        user_email: String,
    ) -> Self {
        CommitContext {
            branch,
            recent_commits,
            staged_files,
            project_metadata,
            user_name,
            user_email,
        }
    }
    pub fn optimize(&mut self, max_tokens: usize) {
        let optimizer = TokenOptimizer::new(max_tokens);
        optimizer.optimize_context(self);
    }
}

pub fn format_commit_message(response: &GeneratedMessage) -> String {
    let mut message = String::new();

    if let Some(emoji) = &response.emoji {
        message.push_str(&format!("{} ", emoji));
    }

    message.push_str(&response.title);
    message.push_str("\n\n");

    let wrapped_message = wrap(&response.message, 78);
    for line in wrapped_message {
        message.push_str(&line);
        message.push_str("\n");
    }

    message
}
