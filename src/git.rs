use crate::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use crate::file_analyzers;
use anyhow::{anyhow, Result};
use git2::{DiffOptions, Repository, StatusOptions};
use std::path::Path;

pub fn get_git_info(repo_path: &Path) -> Result<CommitContext> {
    let repo = Repository::open(repo_path)?;
    let branch = get_current_branch(&repo)?;
    let recent_commits = get_recent_commits(&repo, 5)?;
    let (staged_files, unstaged_files) = get_file_statuses(&repo)?;
    let project_metadata = get_project_metadata(repo_path)?;

    Ok(CommitContext::new(
        branch,
        recent_commits,
        staged_files,
        unstaged_files,
        project_metadata,
    ))
}

fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    Ok(head.shorthand().unwrap_or("HEAD detached").to_string())
}

fn get_recent_commits(repo: &Repository, count: usize) -> Result<Vec<RecentCommit>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let commits = revwalk
        .take(count)
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
        .collect::<Result<Vec<_>>>()?;

    Ok(commits)
}

fn get_file_statuses(repo: &Repository) -> Result<(Vec<StagedFile>, Vec<String>)> {
    let mut staged_files = Vec::new();
    let mut unstaged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().unwrap();
        let status = entry.status();

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            let change_type = if status.is_index_new() {
                ChangeType::Added
            } else if status.is_index_modified() {
                ChangeType::Modified
            } else {
                ChangeType::Deleted
            };

            let diff = get_diff_for_file(repo, path, true)?;
            let analyzer = file_analyzers::get_analyzer(path);
            let staged_file = StagedFile {
                path: path.to_string(),
                change_type: change_type.clone(),
                diff: diff.clone(),
                analysis: Vec::new(),
            };
            let analysis = analyzer.analyze(path, &staged_file);

            staged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
                analysis,
            });
        } else if status.is_wt_modified() || status.is_wt_new() || status.is_wt_deleted() {
            unstaged_files.push(path.to_string());
        }
    }

    Ok((staged_files, unstaged_files))
}

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
        Ok(diff_string)
    }
}

fn is_binary_diff(diff: &str) -> bool {
    diff.contains("Binary files") || diff.contains("GIT binary patch")
}

fn get_project_metadata(repo_path: &Path) -> Result<ProjectMetadata> {
    let mut language = "Unknown".to_string();
    let framework = None;
    let dependencies = Vec::new();

    if repo_path.join("Cargo.toml").exists() {
        language = "Rust".to_string();
        // TODO: Parse Cargo.toml to get dependencies
    } else if repo_path.join("package.json").exists() {
        language = "JavaScript".to_string();
        // TODO: Parse package.json to get dependencies and possibly framework
    } else if repo_path.join("requirements.txt").exists() {
        language = "Python".to_string();
        // TODO: Parse requirements.txt to get dependencies
    }

    // TODO: Implement more sophisticated detection logic

    Ok(ProjectMetadata {
        language,
        framework,
        dependencies,
    })
}

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

pub fn show_file_from_head(repo_path: &Path, file: &str) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    let tree = head_commit.tree()?;
    let entry = tree.get_path(Path::new(file))?;
    let object = entry.to_object(&repo)?;
    let blob = object
        .as_blob()
        .ok_or_else(|| anyhow!("Failed to get blob"))?;
    Ok(String::from_utf8_lossy(blob.content()).to_string())
}

pub fn is_inside_work_tree() -> Result<bool> {
    match Repository::discover(".") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

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
    Ok(())
}
