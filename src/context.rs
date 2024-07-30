use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct CommitContext {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub unstaged_files: Vec<String>,
    pub project_metadata: ProjectMetadata,
}

#[derive(Serialize, Debug)]
pub struct RecentCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

#[derive(Serialize, Debug)]
pub struct StagedFile {
    pub path: String,
    pub change_type: ChangeType,
    pub diff: String,
    pub analysis: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

#[derive(Serialize, Debug)]
pub struct ProjectMetadata {
    pub language: String,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
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
}
