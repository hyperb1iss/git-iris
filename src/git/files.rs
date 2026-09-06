use crate::context::{ChangeType, RecentCommit, StagedFile};
use crate::git::utils::{is_binary_diff, should_exclude_file};
use crate::log_debug;
use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, StatusOptions};
use std::fs;
use std::path::Path;

/// Collects repository information about files and branches
#[derive(Debug)]
pub struct RepoFilesInfo {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub file_paths: Vec<String>,
}

/// Retrieves the status of files in the repository.
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects or an error.
pub fn get_file_statuses(repo: &Repository) -> Result<Vec<StagedFile>> {
    log_debug!("Getting file statuses");
    let mut staged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().context("Could not get path")?;
        let status = entry.status();

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            let change_type = if status.is_index_new() {
                ChangeType::Added
            } else if status.is_index_modified() {
                ChangeType::Modified
            } else {
                ChangeType::Deleted
            };

            let should_exclude = should_exclude_file(path);
            let diff = if should_exclude {
                String::from("[Content excluded]")
            } else {
                get_diff_for_file(repo, path)?
            };

            let content =
                if should_exclude || change_type != ChangeType::Modified || is_binary_diff(&diff) {
                    None
                } else {
                    get_index_content_for_file(repo, path)?
                };

            staged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
                content,
                content_excluded: should_exclude,
            });
        }
    }

    log_debug!("Found {} staged files", staged_files.len());
    Ok(staged_files)
}

/// Retrieves the diff for a specific file.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `path` - The path of the file to get the diff for.
///
/// # Returns
///
/// A Result containing the diff as a String or an error.
pub fn get_diff_for_file(repo: &Repository, path: &str) -> Result<String> {
    log_debug!("Getting diff for file: {}", path);
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(path);

    let tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());
    let index = repo.index()?;
    let diff = repo.diff_tree_to_index(tree.as_ref(), Some(&index), Some(&mut diff_options))?;

    let mut diff_string = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = match line.origin() {
            '+' | '-' | ' ' => line.origin(),
            _ => ' ',
        };
        diff_string.push(origin);
        diff_string.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    if is_binary_diff(&diff_string) {
        Ok("[Binary file changed]".to_string())
    } else {
        log_debug!("Generated diff for {} ({} bytes)", path, diff_string.len());
        Ok(diff_string)
    }
}

fn get_index_content_for_file(repo: &Repository, path: &str) -> Result<Option<String>> {
    let index = repo.index()?;
    let Some(entry) = index.get_path(Path::new(path), 0) else {
        return Ok(None);
    };

    let blob = repo.find_blob(entry.id)?;
    match std::str::from_utf8(blob.content()) {
        Ok(content) => Ok(Some(content.to_string())),
        Err(_) => Ok(None),
    }
}

/// Gets unstaged file changes from the repository
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects for unstaged changes or an error.
pub fn get_unstaged_file_statuses(repo: &Repository) -> Result<Vec<StagedFile>> {
    log_debug!("Getting unstaged file statuses");
    let mut unstaged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().context("Could not get path")?;
        let status = entry.status();

        // Look for changes in the working directory (unstaged)
        if status.is_wt_new() || status.is_wt_modified() || status.is_wt_deleted() {
            let change_type = if status.is_wt_new() {
                ChangeType::Added
            } else if status.is_wt_modified() {
                ChangeType::Modified
            } else {
                ChangeType::Deleted
            };

            let should_exclude = should_exclude_file(path);
            let diff = if should_exclude {
                String::from("[Content excluded]")
            } else {
                get_diff_for_unstaged_file(repo, path)?
            };

            let content =
                if should_exclude || change_type != ChangeType::Modified || is_binary_diff(&diff) {
                    None
                } else {
                    let path_obj = Path::new(path);
                    if path_obj.exists() {
                        Some(fs::read_to_string(path_obj)?)
                    } else {
                        None
                    }
                };

            unstaged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
                content,
                content_excluded: should_exclude,
            });
        }
    }

    log_debug!("Found {} unstaged files", unstaged_files.len());
    Ok(unstaged_files)
}

/// Gets the diff for an unstaged file
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `path` - The path of the file to get the diff for.
///
/// # Returns
///
/// A Result containing the diff as a String or an error.
pub fn get_diff_for_unstaged_file(repo: &Repository, path: &str) -> Result<String> {
    log_debug!("Getting unstaged diff for file: {}", path);
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(path);

    // For unstaged changes, we compare the index (staged) to the working directory
    let diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;

    let mut diff_string = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = match line.origin() {
            '+' | '-' | ' ' => line.origin(),
            _ => ' ',
        };
        diff_string.push(origin);
        diff_string.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    if is_binary_diff(&diff_string) {
        Ok("[Binary file changed]".to_string())
    } else {
        log_debug!(
            "Generated unstaged diff for {} ({} bytes)",
            path,
            diff_string.len()
        );
        Ok(diff_string)
    }
}

/// Gets only untracked files from the repository (new files not in the index)
///
/// # Returns
///
/// A Result containing a Vec of file paths for untracked files or an error.
pub fn get_untracked_files(repo: &Repository) -> Result<Vec<String>> {
    log_debug!("Getting untracked files");
    let mut untracked = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let status = entry.status();
        // Only include files that are untracked (not in index, not ignored)
        if status.is_wt_new()
            && !status.is_index_new()
            && let Ok(path) = entry.path()
        {
            untracked.push(path.to_string());
        }
    }

    log_debug!("Found {} untracked files", untracked.len());
    Ok(untracked)
}

/// Gets all tracked files in the repository (from HEAD tree + index)
///
/// This returns all files that are tracked by git, which includes:
/// - Files committed in HEAD
/// - Files staged in the index (including newly added files)
///
/// # Returns
///
/// A Result containing a Vec of file paths or an error.
pub fn get_all_tracked_files(repo: &Repository) -> Result<Vec<String>> {
    log_debug!("Getting all tracked files");
    let mut files = std::collections::HashSet::new();

    // Get files from HEAD tree
    if let Ok(head) = repo.head()
        && let Ok(tree) = head.peel_to_tree()
    {
        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let path = if dir.is_empty() {
                    entry.name().unwrap_or("").to_string()
                } else {
                    format!("{}{}", dir, entry.name().unwrap_or(""))
                };
                if !path.is_empty() {
                    files.insert(path);
                }
            }
            git2::TreeWalkResult::Ok
        })?;
    }

    // Also include files from the index (staged files, including new files)
    let index = repo.index()?;
    for entry in index.iter() {
        let path = String::from_utf8_lossy(&entry.path).to_string();
        files.insert(path);
    }

    let mut result: Vec<_> = files.into_iter().collect();
    result.sort();

    log_debug!("Found {} tracked files", result.len());
    Ok(result)
}

/// Gets the number of commits ahead and behind the upstream tracking branch
///
/// # Returns
///
/// A tuple of (ahead, behind) counts, or (0, 0) if no upstream
pub fn get_ahead_behind(repo: &Repository) -> (usize, usize) {
    log_debug!("Getting ahead/behind counts");

    // Get the current branch
    let Ok(head) = repo.head() else {
        return (0, 0); // No HEAD
    };

    let Ok(branch_name) = head.shorthand() else {
        return (0, 0);
    };

    // Try to find the upstream branch
    let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Local) else {
        return (0, 0);
    };

    let Ok(upstream) = branch.upstream() else {
        return (0, 0); // No upstream configured
    };

    // Get the OIDs for local and upstream
    let Some(local_oid) = head.target() else {
        return (0, 0);
    };

    let Some(upstream_oid) = upstream.get().target() else {
        return (0, 0);
    };

    // Calculate ahead/behind
    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok((ahead, behind)) => {
            log_debug!("Branch is {} ahead, {} behind upstream", ahead, behind);
            (ahead, behind)
        }
        Err(_) => (0, 0),
    }
}
