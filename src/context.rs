use serde::Serialize;
use std::fmt;

use crate::token_optimizer::TokenOptimizer;

#[derive(Serialize, Debug, Clone)]
pub struct CommitContext {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub unstaged_files: Vec<String>,
    pub project_metadata: ProjectMetadata,
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
    pub content_excluded: bool,
}

#[derive(Serialize, Debug, Clone)]
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
        unstaged_files: Vec<String>,
        project_metadata: ProjectMetadata,
    ) -> Self {
        CommitContext {
            branch,
            recent_commits,
            staged_files,
            unstaged_files,
            project_metadata,
        }
    }
    pub fn optimize(&mut self, max_tokens: usize) {
        let optimizer = TokenOptimizer::new(max_tokens);
        optimizer.optimize_context(self);
    }
}
