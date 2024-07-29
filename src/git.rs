use crate::log_debug;
use anyhow::{anyhow, Result};
use git2::{DiffOptions, Repository, Status, StatusOptions};
use std::collections::HashMap;
use std::path::Path;

/// Structure representing information about a Git repository
#[derive(Debug)]
pub struct GitInfo {
    pub branch: String,
    pub recent_commits: Vec<String>,
    pub staged_files: HashMap<String, FileChange>,
    pub unstaged_files: Vec<String>,
    pub project_root: String,
}

/// Structure representing changes in a file
#[derive(Debug, Clone)]
pub struct FileChange {
    pub status: String,
    pub diff: String,
}

/// Get information about the current Git repository
pub fn get_git_info(repo_path: &Path) -> Result<GitInfo> {
    let repo = Repository::open(repo_path)?;
    let branch = get_current_branch(&repo)?;
    let recent_commits = get_recent_commits(&repo, 5)?;
    let (staged_files, unstaged_files) = get_file_statuses(&repo)?;
    let project_root = repo.path().parent().unwrap().to_str().unwrap().to_string();

    log_debug!(
        "Git information retrieved: {:?}",
        GitInfo {
            branch: branch.clone(),
            recent_commits: recent_commits.clone(),
            staged_files: staged_files.clone(),
            unstaged_files: unstaged_files.clone(),
            project_root: project_root.clone(),
        }
    );

    Ok(GitInfo {
        branch,
        recent_commits,
        staged_files,
        unstaged_files,
        project_root,
    })
}

/// Get the name of the current branch
fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| anyhow!("Failed to get branch name"))?;
    Ok(branch_name.to_string())
}

/// Get a list of recent commits
fn get_recent_commits(repo: &Repository, count: usize) -> Result<Vec<String>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let commits: Result<Vec<_>> = revwalk
        .take(count)
        .map(|oid| {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            Ok(format!(
                "{} {}",
                &oid.to_string()[..7],
                commit.summary().unwrap_or_default()
            ))
        })
        .collect();

    commits
}

/// Get the status of staged and unstaged files
fn get_file_statuses(repo: &Repository) -> Result<(HashMap<String, FileChange>, Vec<String>)> {
    let mut staged_files = HashMap::new();
    let mut unstaged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().unwrap();
        let status = entry.status();

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            let diff = get_diff_for_file(repo, path, true)?;
            staged_files.insert(
                path.to_string(),
                FileChange {
                    status: status_to_string(status),
                    diff,
                },
            );
        } else if status.is_wt_modified() || status.is_wt_new() || status.is_wt_deleted() {
            unstaged_files.push(path.to_string());
        }
    }

    Ok((staged_files, unstaged_files))
}

/// Convert a git2::Status to a human-readable string
fn status_to_string(status: Status) -> String {
    if status.is_index_new() {
        "A".to_string()
    } else if status.is_index_modified() {
        "M".to_string()
    } else if status.is_index_deleted() {
        "D".to_string()
    } else {
        "?".to_string()
    }
}

/// Get the diff for a specific file
fn get_diff_for_file(repo: &Repository, path: &str, staged: bool) -> Result<String> {
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(path);

    let tree = if staged {
        Some(repo.head()?.peel_to_tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut diff_options))?;

    let mut diff_string = String::new();
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        diff_string.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    if is_binary_diff(&diff_string) {
        log_debug!("Binary file detected: {}", path);
        Ok("[Binary file changed]".to_string())
    } else {
        Ok(diff_string)
    }
}

/// Check if the diff string indicates a binary file
fn is_binary_diff(diff: &str) -> bool {
    diff.contains("Binary files") || diff.contains("GIT binary patch")
}

/// Check if Git is installed and accessible
pub fn check_environment() -> Result<()> {
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_err()
    {
        return Err(anyhow!("Git is not installed or not in the PATH"));
    }

    Ok(())
}

/// Check if the current directory is inside a Git work tree
pub fn is_inside_work_tree() -> Result<bool> {
    // Example: Check if we're inside a Git repository
    match Repository::discover(".") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Commit changes to the repository with the given message
pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let signature = repo.signature()?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;
    log_debug!("Commit successful with message: {}", message);
    Ok(())
}
