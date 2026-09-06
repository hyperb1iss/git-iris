use crate::context::{ChangeType, RecentCommit, StagedFile};
use crate::git::utils::{is_binary_diff, should_exclude_file};
use crate::log_debug;
use anyhow::{Result, anyhow};
use chrono;
use git2::{Delta, FileMode, Repository};

/// Results from a commit operation
#[derive(Debug)]
pub struct CommitResult {
    pub branch: String,
    pub commit_hash: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub new_files: Vec<(String, FileMode)>,
}

/// Collects information about a specific commit
#[derive(Debug)]
pub struct CommitInfo {
    pub branch: String,
    pub commit: RecentCommit,
    pub file_paths: Vec<String>,
}

/// Commits changes to the repository.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `message` - The commit message.
/// * `is_remote` - Whether the repository is remote.
///
/// # Returns
///
/// A Result containing the `CommitResult` or an error.
pub fn commit(repo: &Repository, message: &str, is_remote: bool) -> Result<CommitResult> {
    if is_remote {
        return Err(anyhow!(
            "Cannot commit to a remote repository in read-only mode"
        ));
    }

    let signature = repo.signature()?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent_commit = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
        Err(error) => return Err(error.into()),
    };
    let parents: Vec<_> = parent_commit.iter().collect();
    let commit_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )?;

    let branch_name = repo.head()?.shorthand().unwrap_or("HEAD").to_string();
    let commit = repo.find_commit(commit_oid)?;
    let short_hash = commit.id().to_string()[..7].to_string();

    let parent_tree = parent_commit.as_ref().map(git2::Commit::tree).transpose()?;
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    let stats = diff.stats()?;

    Ok(CommitResult {
        branch: branch_name,
        commit_hash: short_hash,
        files_changed: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        new_files: added_files(&diff),
    })
}

/// Amends the previous commit with staged changes and a new message.
///
/// This replaces HEAD with a new commit that has:
/// - HEAD's parent as its parent
/// - The current staged index as its tree
/// - The new message provided
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `message` - The new commit message
/// * `is_remote` - Whether the repository is remote
///
/// # Returns
///
/// A Result containing the `CommitResult` or an error.
pub fn amend_commit(repo: &Repository, message: &str, is_remote: bool) -> Result<CommitResult> {
    if is_remote {
        return Err(anyhow!(
            "Cannot amend a commit in a remote repository in read-only mode"
        ));
    }

    let signature = repo.signature()?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Get the current HEAD commit (the one we're amending)
    let head_commit = repo.head()?.peel_to_commit()?;

    // Amend the HEAD commit with the new tree and message
    let commit_oid = head_commit.amend(
        Some("HEAD"),     // Update the HEAD reference
        None,             // Preserve the original author and author date
        Some(&signature), // New committer (use current)
        None,             // Keep original encoding
        Some(message),    // New message
        Some(&tree),      // New tree (includes staged changes)
    )?;

    let branch_name = repo.head()?.shorthand().unwrap_or("HEAD").to_string();
    let commit = repo.find_commit(commit_oid)?;
    let short_hash = commit.id().to_string()[..7].to_string();

    // Use the first parent for diff (or empty tree if initial commit)
    let parent_tree = if head_commit.parent_count() > 0 {
        Some(head_commit.parent(0)?.tree()?)
    } else {
        None
    };
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

    let stats = diff.stats()?;

    log_debug!(
        "Amended commit {} -> {} with {} files changed",
        &head_commit.id().to_string()[..7],
        short_hash,
        stats.files_changed()
    );

    Ok(CommitResult {
        branch: branch_name,
        commit_hash: short_hash,
        files_changed: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        new_files: added_files(&diff),
    })
}

fn added_files(diff: &git2::Diff<'_>) -> Vec<(String, FileMode)> {
    diff.deltas()
        .filter(|delta| delta.status() == Delta::Added)
        .filter_map(|delta| {
            let file = delta.new_file();
            Some((file.path()?.to_string_lossy().into_owned(), file.mode()))
        })
        .collect()
}

/// Gets the message of the HEAD commit.
///
/// # Arguments
///
/// * `repo` - The git repository
///
/// # Returns
///
/// A Result containing the commit message or an error.
pub fn get_head_commit_message(repo: &Repository) -> Result<String> {
    let head_commit = repo.head()?.peel_to_commit()?;
    Ok(head_commit.message().unwrap_or_default().to_string())
}

/// Retrieves commits between two Git references.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `from` - The starting Git reference.
/// * `to` - The ending Git reference.
/// * `callback` - A callback function to process each commit.
///
/// # Returns
///
/// A Result containing a Vec of processed commits or an error.
pub fn get_commits_between_with_callback<T, F>(
    repo: &Repository,
    from: &str,
    to: &str,
    mut callback: F,
) -> Result<Vec<T>>
where
    F: FnMut(&RecentCommit) -> Result<T>,
{
    let from_commit = repo.revparse_single(from)?.peel_to_commit()?;
    let to_commit = repo.revparse_single(to)?.peel_to_commit()?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push(to_commit.id())?;
    revwalk.hide(from_commit.id())?;

    revwalk
        .filter_map(std::result::Result::ok)
        .map(|id| {
            let commit = repo.find_commit(id)?;
            let recent_commit = RecentCommit {
                hash: commit.id().to_string(),
                message: commit.message().unwrap_or_default().to_string(),
                author: commit.author().name().unwrap_or_default().to_string(),
                timestamp: commit.time().seconds().to_string(),
            };
            callback(&recent_commit)
        })
        .collect()
}

/// Retrieves the files changed in a specific commit
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `commit_id` - The ID of the commit to analyze.
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects for the commit or an error.
pub fn get_commit_files(repo: &Repository, commit_id: &str) -> Result<Vec<StagedFile>> {
    log_debug!("Getting files for commit: {}", commit_id);

    // Parse the commit ID
    let obj = repo.revparse_single(commit_id)?;
    let commit = obj.peel_to_commit()?;

    let commit_tree = commit.tree()?;
    let parent_commit = if commit.parent_count() > 0 {
        Some(commit.parent(0)?)
    } else {
        None
    };

    let parent_tree = parent_commit.map(|c| c.tree()).transpose()?;

    let mut commit_files = Vec::new();

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;

    // Get statistics for each file and convert to our StagedFile format
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                let change_type = match delta.status() {
                    git2::Delta::Added => ChangeType::Added,
                    git2::Delta::Modified => ChangeType::Modified,
                    git2::Delta::Deleted => ChangeType::Deleted,
                    _ => return true, // Skip other types of changes
                };

                let should_exclude = should_exclude_file(path);

                commit_files.push(StagedFile {
                    path: path.to_string(),
                    change_type,
                    diff: String::new(), // Will be populated later
                    content: None,
                    content_excluded: should_exclude,
                });
            }
            true
        },
        None,
        None,
        None,
    )?;

    // Get the diff for each file
    for file in &mut commit_files {
        if file.content_excluded {
            file.diff = String::from("[Content excluded]");
            continue;
        }

        let mut diff_options = git2::DiffOptions::new();
        diff_options.pathspec(&file.path);

        let file_diff = repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_options),
        )?;

        let mut diff_string = String::new();
        file_diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = match line.origin() {
                '+' | '-' | ' ' => line.origin(),
                _ => ' ',
            };
            diff_string.push(origin);
            diff_string.push_str(&String::from_utf8_lossy(line.content()));
            true
        })?;

        if is_binary_diff(&diff_string) {
            file.diff = "[Binary file changed]".to_string();
        } else {
            file.diff = diff_string;
        }
    }

    log_debug!("Found {} files in commit", commit_files.len());
    Ok(commit_files)
}

/// Extract commit info without crossing async boundaries
pub fn extract_commit_info(repo: &Repository, commit_id: &str, branch: &str) -> Result<CommitInfo> {
    // Parse the commit ID
    let obj = repo.revparse_single(commit_id)?;
    let commit = obj.peel_to_commit()?;

    // Extract commit information
    let commit_author = commit.author();
    let author_name = commit_author.name().unwrap_or_default().to_string();
    let commit_message = commit.message().unwrap_or_default().to_string();
    let commit_time = commit.time().seconds().to_string();
    let commit_hash = commit.id().to_string();

    // Create the recent commit object
    let recent_commit = RecentCommit {
        hash: commit_hash,
        message: commit_message,
        author: author_name,
        timestamp: commit_time,
    };

    // Get file paths from this commit
    let file_paths = get_file_paths_for_commit(repo, commit_id)?;

    Ok(CommitInfo {
        branch: branch.to_string(),
        commit: recent_commit,
        file_paths,
    })
}

/// Gets just the file paths for a specific commit (not the full content)
pub fn get_file_paths_for_commit(repo: &Repository, commit_id: &str) -> Result<Vec<String>> {
    // Parse the commit ID
    let obj = repo.revparse_single(commit_id)?;
    let commit = obj.peel_to_commit()?;

    let commit_tree = commit.tree()?;
    let parent_commit = if commit.parent_count() > 0 {
        Some(commit.parent(0)?)
    } else {
        None
    };

    let parent_tree = parent_commit.map(|c| c.tree()).transpose()?;

    let mut file_paths = Vec::new();

    // Create diff between trees
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;

    // Extract file paths
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                match delta.status() {
                    git2::Delta::Added | git2::Delta::Modified | git2::Delta::Deleted => {
                        file_paths.push(path.to_string());
                    }
                    _ => {} // Skip other types of changes
                }
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(file_paths)
}

/// Gets the date of a commit in YYYY-MM-DD format
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `commit_ish` - A commit-ish reference (hash, tag, branch, etc.)
///
/// # Returns
///
/// A Result containing the formatted date string or an error
pub fn get_commit_date(repo: &Repository, commit_ish: &str) -> Result<String> {
    // Resolve the commit-ish to an actual commit
    let obj = repo.revparse_single(commit_ish)?;
    let commit = obj.peel_to_commit()?;

    // Get the commit time
    let time = commit.time();

    // Convert to a chrono::DateTime for easier formatting
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(time.seconds(), 0)
        .ok_or_else(|| anyhow!("Invalid timestamp"))?;

    // Format as YYYY-MM-DD
    Ok(datetime.format("%Y-%m-%d").to_string())
}

/// Gets the files changed between two branches
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `base_branch` - The base branch (e.g., "main")
/// * `target_branch` - The target branch (e.g., "feature-branch")
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects for the branch comparison or an error.
pub fn get_branch_diff_files(
    repo: &Repository,
    base_branch: &str,
    target_branch: &str,
) -> Result<Vec<StagedFile>> {
    log_debug!(
        "Getting files changed between branches: {} -> {}",
        base_branch,
        target_branch
    );

    // Resolve branch references
    let base_commit = repo.revparse_single(base_branch)?.peel_to_commit()?;
    let target_commit = repo.revparse_single(target_branch)?.peel_to_commit()?;

    // Find the merge-base (common ancestor) between the branches
    // This gives us the point where the target branch diverged from the base branch
    let merge_base_oid = repo.merge_base(base_commit.id(), target_commit.id())?;
    let merge_base_commit = repo.find_commit(merge_base_oid)?;

    log_debug!("Using merge-base {} for comparison", merge_base_oid);

    let base_tree = merge_base_commit.tree()?;
    let target_tree = target_commit.tree()?;

    let mut branch_files = Vec::new();

    // Create diff between the merge-base tree and target tree
    // This shows only changes made in the target branch since it diverged
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&target_tree), None)?;
    diff.foreach(
        &mut |delta, _| collect_delta_file(&delta, &mut branch_files),
        None,
        None,
        None,
    )?;

    // Get the diff for each file
    for file in &mut branch_files {
        populate_branch_file(repo, &base_tree, &target_tree, file)?;
    }

    log_debug!(
        "Found {} files changed between branches (using merge-base)",
        branch_files.len()
    );
    Ok(branch_files)
}

fn collect_delta_file(delta: &git2::DiffDelta<'_>, branch_files: &mut Vec<StagedFile>) -> bool {
    if let Some(file) = staged_file_from_delta(delta) {
        branch_files.push(file);
    }
    true
}

fn staged_file_from_delta(delta: &git2::DiffDelta<'_>) -> Option<StagedFile> {
    let path = delta.new_file().path()?.to_str()?;
    let change_type = change_type_from_delta(delta.status())?;

    Some(StagedFile {
        path: path.to_string(),
        change_type,
        diff: String::new(),
        content: None,
        content_excluded: should_exclude_file(path),
    })
}

fn change_type_from_delta(delta: git2::Delta) -> Option<ChangeType> {
    match delta {
        git2::Delta::Added => Some(ChangeType::Added),
        git2::Delta::Modified => Some(ChangeType::Modified),
        git2::Delta::Deleted => Some(ChangeType::Deleted),
        _ => None,
    }
}

fn populate_branch_file(
    repo: &Repository,
    base_tree: &git2::Tree<'_>,
    target_tree: &git2::Tree<'_>,
    file: &mut StagedFile,
) -> Result<()> {
    file.diff = branch_file_diff(repo, base_tree, target_tree, file)?;
    file.content = branch_file_content(repo, target_tree, file);
    Ok(())
}

fn branch_file_diff(
    repo: &Repository,
    base_tree: &git2::Tree<'_>,
    target_tree: &git2::Tree<'_>,
    file: &StagedFile,
) -> Result<String> {
    if file.content_excluded {
        return Ok(String::from("[Content excluded]"));
    }

    let mut diff_options = git2::DiffOptions::new();
    diff_options.pathspec(&file.path);

    let file_diff =
        repo.diff_tree_to_tree(Some(base_tree), Some(target_tree), Some(&mut diff_options))?;
    let mut diff_string = String::new();
    file_diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        diff_string.push(patch_line_origin(line.origin()));
        diff_string.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    if is_binary_diff(&diff_string) {
        Ok("[Binary file changed]".to_string())
    } else {
        Ok(diff_string)
    }
}

fn patch_line_origin(origin: char) -> char {
    match origin {
        '+' | '-' | ' ' => origin,
        _ => ' ',
    }
}

fn branch_file_content(
    repo: &Repository,
    target_tree: &git2::Tree<'_>,
    file: &StagedFile,
) -> Option<String> {
    if !matches!(file.change_type, ChangeType::Added | ChangeType::Modified) {
        return None;
    }

    target_tree
        .get_path(std::path::Path::new(&file.path))
        .ok()
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| object.as_blob().map(|blob| blob.content().to_vec()))
        .and_then(|content| String::from_utf8(content).ok())
}

/// Extract branch comparison info without crossing async boundaries
pub fn extract_branch_diff_info(
    repo: &Repository,
    base_branch: &str,
    target_branch: &str,
) -> Result<(String, Vec<RecentCommit>, Vec<String>)> {
    // Get the target branch name for display
    let display_branch = format!("{base_branch} -> {target_branch}");

    // Get commits between the branches using merge-base
    let base_commit = repo.revparse_single(base_branch)?.peel_to_commit()?;
    let target_commit = repo.revparse_single(target_branch)?.peel_to_commit()?;

    // Find the merge-base (common ancestor) between the branches
    let merge_base_oid = repo.merge_base(base_commit.id(), target_commit.id())?;
    log_debug!("Using merge-base {} for commit history", merge_base_oid);

    let mut revwalk = repo.revwalk()?;
    revwalk.push(target_commit.id())?;
    revwalk.hide(merge_base_oid)?; // Hide the merge-base commit itself

    let recent_commits: Result<Vec<RecentCommit>> = revwalk
        .take(10) // Limit to 10 most recent commits in the branch
        .map(|oid| {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            let author = commit.author();
            Ok(RecentCommit {
                hash: oid.to_string(),
                message: commit.message().unwrap_or_default().to_string(),
                author: author.name().unwrap_or_default().to_string(),
                timestamp: commit.time().seconds().to_string(),
            })
        })
        .collect();

    let recent_commits = recent_commits?;

    // Get file paths from the diff for metadata
    let diff_files = get_branch_diff_files(repo, base_branch, target_branch)?;
    let file_paths: Vec<String> = diff_files.iter().map(|file| file.path.clone()).collect();

    Ok((display_branch, recent_commits, file_paths))
}

/// Gets commits between two references with their messages for PR descriptions
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `from` - The starting Git reference (exclusive)
/// * `to` - The ending Git reference (inclusive)
///
/// # Returns
///
/// A Result containing a Vec of formatted commit messages or an error.
pub fn get_commits_for_pr(repo: &Repository, from: &str, to: &str) -> Result<Vec<String>> {
    log_debug!("Getting commits for PR between {} and {}", from, to);

    let from_commit = repo.revparse_single(from)?.peel_to_commit()?;
    let to_commit = repo.revparse_single(to)?.peel_to_commit()?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push(to_commit.id())?;
    revwalk.hide(from_commit.id())?;

    let commits: Result<Vec<String>> = revwalk
        .map(|oid| {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            let message = commit.message().unwrap_or_default();
            // Get just the first line (title) of the commit message
            let title = message.lines().next().unwrap_or_default();
            Ok(format!("{}: {}", &oid.to_string()[..7], title))
        })
        .collect();

    let mut result = commits?;
    result.reverse(); // Show commits in chronological order

    log_debug!("Found {} commits for PR", result.len());
    Ok(result)
}

/// Gets the files changed in a commit range (similar to branch diff but for commit range)
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `from` - The starting Git reference (exclusive)
/// * `to` - The ending Git reference (inclusive)
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects for the commit range or an error.
pub fn get_commit_range_files(repo: &Repository, from: &str, to: &str) -> Result<Vec<StagedFile>> {
    log_debug!("Getting files changed in commit range: {} -> {}", from, to);

    // Resolve commit references
    let from_commit = repo.revparse_single(from)?.peel_to_commit()?;
    let to_commit = repo.revparse_single(to)?.peel_to_commit()?;

    let from_tree = from_commit.tree()?;
    let to_tree = to_commit.tree()?;

    let mut range_files = Vec::new();

    // Create diff between the from and to trees
    let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;

    // Get statistics for each file and convert to our StagedFile format
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                let change_type = match delta.status() {
                    git2::Delta::Added => ChangeType::Added,
                    git2::Delta::Modified => ChangeType::Modified,
                    git2::Delta::Deleted => ChangeType::Deleted,
                    _ => return true, // Skip other types of changes
                };

                let should_exclude = should_exclude_file(path);

                range_files.push(StagedFile {
                    path: path.to_string(),
                    change_type,
                    diff: String::new(), // Will be populated later
                    content: None,
                    content_excluded: should_exclude,
                });
            }
            true
        },
        None,
        None,
        None,
    )?;

    // Get the diff for each file
    for file in &mut range_files {
        if file.content_excluded {
            file.diff = String::from("[Content excluded]");
            continue;
        }

        let mut diff_options = git2::DiffOptions::new();
        diff_options.pathspec(&file.path);

        let file_diff =
            repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_options))?;

        let mut diff_string = String::new();
        file_diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = match line.origin() {
                '+' | '-' | ' ' => line.origin(),
                _ => ' ',
            };
            diff_string.push(origin);
            diff_string.push_str(&String::from_utf8_lossy(line.content()));
            true
        })?;

        if is_binary_diff(&diff_string) {
            file.diff = "[Binary file changed]".to_string();
        } else {
            file.diff = diff_string;
        }

        // Get file content from to commit if it's a modified or added file
        if matches!(file.change_type, ChangeType::Added | ChangeType::Modified)
            && let Ok(entry) = to_tree.get_path(std::path::Path::new(&file.path))
            && let Ok(object) = entry.to_object(repo)
            && let Some(blob) = object.as_blob()
            && let Ok(content) = std::str::from_utf8(blob.content())
        {
            file.content = Some(content.to_string());
        }
    }

    log_debug!("Found {} files changed in commit range", range_files.len());
    Ok(range_files)
}

/// Extract commit range info without crossing async boundaries
pub fn extract_commit_range_info(
    repo: &Repository,
    from: &str,
    to: &str,
) -> Result<(String, Vec<RecentCommit>, Vec<String>)> {
    // Get the range name for display
    let display_range = format!("{from}..{to}");

    // Get commits in the range
    let recent_commits: Result<Vec<RecentCommit>> =
        get_commits_between_with_callback(repo, from, to, |commit| Ok(commit.clone()));
    let recent_commits = recent_commits?;

    // Get file paths from the range for metadata
    let range_files = get_commit_range_files(repo, from, to)?;
    let file_paths: Vec<String> = range_files.iter().map(|file| file.path.clone()).collect();

    Ok((display_range, recent_commits, file_paths))
}
