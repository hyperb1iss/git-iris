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
    #[allow(clippy::unwrap_used)] // todo: handle error maybe replace with try_from
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap()
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "Added"),
            Self::Modified => write!(f, "Modified"),
            Self::Deleted => write!(f, "Deleted"),
        }
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct ProjectMetadata {
    pub language: Option<String>,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub version: Option<String>,
    pub build_system: Option<String>,
    pub test_framework: Option<String>,
    pub plugins: Vec<String>,
}

impl ProjectMetadata {
    pub fn merge(&mut self, new: ProjectMetadata) {
        if let Some(new_lang) = new.language {
            match &mut self.language {
                Some(lang) if !lang.contains(&new_lang) => {
                    lang.push_str(", ");
                    lang.push_str(&new_lang);
                }
                None => self.language = Some(new_lang),
                _ => {}
            }
        }
        self.dependencies.extend(new.dependencies.clone());
        self.framework = self.framework.take().or(new.framework);
        self.version = self.version.take().or(new.version);
        self.build_system = self.build_system.take().or(new.build_system);
        self.test_framework = self.test_framework.take().or(new.test_framework);
        self.plugins.extend(new.plugins);
        self.dependencies.sort();
        self.dependencies.dedup();
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
        Self {
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
        message.push_str(&format!("{emoji} "));
    }

    message.push_str(&response.title);
    message.push_str("\n\n");

    let wrapped_message = wrap(&response.message, 78);
    for line in wrapped_message {
        message.push_str(&line);
        message.push('\n');
    }

    message
}
