use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the structured response for a changelog
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ChangelogResponse {
    /// The version number of the release
    pub version: Option<String>,
    /// The date of the release
    pub release_date: Option<String>,
    /// Categorized changes, grouped by type
    pub sections: HashMap<ChangelogType, Vec<ChangeEntry>>,
    /// List of breaking changes in this release
    pub breaking_changes: Vec<BreakingChange>,
    /// Metrics summarizing the changes in this release
    pub metrics: ChangeMetrics,
}

/// Enumeration of possible change types for changelog entries
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq, Hash)]
pub enum ChangelogType {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
}

/// Represents a single change entry in the changelog
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ChangeEntry {
    /// Description of the change
    pub description: String,
    /// List of commit hashes associated with this change
    pub commit_hashes: Vec<String>,
    /// List of issue numbers associated with this change
    pub associated_issues: Vec<String>,
    /// Pull request number associated with this change, if any
    pub pull_request: Option<String>,
}

/// Represents a breaking change in the release
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct BreakingChange {
    /// Description of the breaking change
    pub description: String,
    /// Commit hash associated with this breaking change
    pub commit_hash: String,
}

/// Metrics summarizing the changes in a release
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct ChangeMetrics {
    /// Total number of commits in this release
    pub total_commits: usize,
    /// Number of files changed in this release
    pub files_changed: usize,
    /// Number of lines inserted in this release
    pub insertions: usize,
    /// Number of lines deleted in this release
    pub deletions: usize,
    /// Total lines changed in this release
    pub total_lines_changed: usize,
}

/// Represents the structured response for release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ReleaseNotesResponse {
    /// The version number of the release
    pub version: Option<String>,
    /// The date of the release
    pub release_date: Option<String>,
    /// A brief summary of the release
    pub summary: String,
    /// List of highlighted changes or features in this release
    pub highlights: Vec<Highlight>,
    /// Detailed sections of changes
    pub sections: Vec<Section>,
    /// List of breaking changes in this release
    pub breaking_changes: Vec<BreakingChange>,
    /// Notes for upgrading to this version
    pub upgrade_notes: Vec<String>,
    /// Metrics summarizing the changes in this release
    pub metrics: ChangeMetrics,
}

/// Represents a highlight in the release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Highlight {
    /// Title of the highlight
    pub title: String,
    /// Detailed description of the highlight
    pub description: String,
}

/// Represents a section in the release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Section {
    /// Title of the section
    pub title: String,
    /// List of items in this section
    pub items: Vec<SectionItem>,
}

/// Represents an item in a section of the release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct SectionItem {
    /// Description of the change
    pub description: String,
    /// List of issue numbers associated with this change
    pub associated_issues: Vec<String>,
    /// Pull request number associated with this change, if any
    pub pull_request: Option<String>,
}

impl From<String> for ChangelogResponse {
    /// Converts a JSON string to a ChangelogResponse
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap()
    }
}

impl From<String> for ReleaseNotesResponse {
    /// Converts a JSON string to a ReleaseNotesResponse
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap()
    }
}