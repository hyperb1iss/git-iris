//! Agent tools module
//!
//! This module contains all the tools available to Iris for performing various operations.
//! Each tool implements Rig's Tool trait for proper integration.

// Common utilities shared across tools
pub mod common;
pub use common::{
    current_repo_root, get_current_repo, parameters_schema, with_active_repo_root,
    with_repo_execution_context,
};

// Tool registry for consistent attachment
pub mod registry;
pub use registry::CORE_TOOLS;

// Tool modules with Rig-based implementations
pub mod git;

// Re-export the tool structs (not functions) for Rig agents
pub use git::{GitBlame, GitChangedFiles, GitDiff, GitLog, GitRepoInfo, GitShow, GitStatus};

// Migrated Rig tools
pub mod file_read;
pub use file_read::FileRead;

pub mod code_search;
pub use code_search::CodeSearch;

pub mod docs;
pub use docs::ProjectDocs;

pub mod repo_map;
pub use repo_map::{RepoMap, RepoMapArgs, RepoMapTool};

pub mod static_analysis;
pub use static_analysis::{StaticAnalysis, StaticAnalysisArgs, StaticAnalyzer};

pub mod workspace;
pub use workspace::Workspace;

pub mod parallel_analyze;
pub use parallel_analyze::{ParallelAnalyze, ParallelAnalyzeResult, SubagentResult};

pub mod content_update;
pub use content_update::{
    ContentUpdate, ContentUpdateReceiver, ContentUpdateSender, UpdateCommitTool, UpdatePRTool,
    UpdateReviewTool, create_content_update_channel,
};

#[cfg(test)]
mod tests;
