use crate::changes::change_analyzer::{AnalyzedChange, ChangeAnalyzer};
use crate::config::Config;
use crate::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use crate::file_analyzers::{self, FileAnalyzer};
use crate::log_debug;
use anyhow::{anyhow, Result};
use futures::future::join_all;
use git2::{DiffOptions, FileMode, Repository, Status, StatusOptions};
use regex::Regex;
use std::fs;
use std::path::Path;
use tokio::task;

#[derive(Debug)]
pub struct CommitResult {
    pub branch: String,
    pub commit_hash: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub new_files: Vec<(String, FileMode)>, // (file_path, file_mode)
}

pub async fn get_git_info(repo_path: &Path, _config: &Config) -> Result<CommitContext> {
    log_debug!("Getting git info for repo path: {:?}", repo_path);
    let repo = Repository::open(repo_path)?;

    let branch = get_current_branch(&repo)?;
    let recent_commits = get_recent_commits(&repo, 5)?;
    let staged_files = get_file_statuses(&repo)?;

    // Get the list of changed file paths
    let changed_files: Vec<String> = staged_files.iter().map(|file| file.path.clone()).collect();

    log_debug!("Changed files for metadata extraction: {:?}", changed_files);

    let project_metadata = get_project_metadata(&changed_files).await?;

    log_debug!("Extracted project metadata: {:?}", project_metadata);

    let user_name = repo.config()?.get_string("user.name")?;
    let user_email = repo.config()?.get_string("user.email")?;

    let context = CommitContext::new(
        branch,
        recent_commits,
        staged_files,
        project_metadata,
        user_name,
        user_email,
    );

    log_debug!("Git info retrieved successfully");
    Ok(context)
}

fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD detached").to_string();
    log_debug!("Current branch: {}", branch_name);
    Ok(branch_name)
}

fn get_recent_commits(repo: &Repository, count: usize) -> Result<Vec<RecentCommit>> {
    log_debug!("Fetching {} recent commits", count);
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

    log_debug!("Retrieved {} recent commits", commits.len());
    Ok(commits)
}

pub fn get_commits_between(repo_path: &Path, from: &str, to: &str) -> Result<Vec<AnalyzedChange>> {
    log_debug!("Analyzing commits between '{}' and '{}'", from, to);
    let repo = Repository::open(repo_path)?;
    let analyzer = ChangeAnalyzer::new(&repo);

    let from_commit = repo.revparse_single(from)?.peel_to_commit()?;
    let to_commit = repo.revparse_single(to)?.peel_to_commit()?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push(to_commit.id())?;
    revwalk.hide(from_commit.id())?;

    let analyzed_commits: Vec<AnalyzedChange> = revwalk
        .filter_map(|id| id.ok())
        .filter_map(|id| repo.find_commit(id).ok())
        .filter_map(|commit| analyzer.analyze_commit(&commit).ok())
        .collect();

    log_debug!(
        "Analyzed {} commits between '{}' and '{}'",
        analyzed_commits.len(),
        from,
        to
    );
    Ok(analyzed_commits)
}

fn should_exclude_file(path: &str) -> bool {
    log_debug!("Checking if file should be excluded: {}", path);
    let exclude_patterns = vec![
        (String::from(r"\.git"), false),
        (String::from(r"\.svn"), false),
        (String::from(r"\.hg"), false),
        (String::from(r"\.DS_Store"), false),
        (String::from(r"node_modules"), false),
        (String::from(r"target"), false),
        (String::from(r"build"), false),
        (String::from(r"dist"), false),
        (String::from(r"\.vscode"), false),
        (String::from(r"\.idea"), false),
        (String::from(r"\.vs"), false),
        (String::from(r"package-lock\.json$"), true),
        (String::from(r"\.lock$"), true),
        (String::from(r"\.log$"), true),
        (String::from(r"\.tmp$"), true),
        (String::from(r"\.temp$"), true),
        (String::from(r"\.swp$"), true),
        (String::from(r"\.min\.js$"), true),
    ];

    let path = Path::new(path);

    for (pattern, is_extension) in exclude_patterns {
        let re = Regex::new(&pattern).unwrap();
        if is_extension {
            if let Some(file_name) = path.file_name() {
                if re.is_match(file_name.to_str().unwrap()) {
                    log_debug!("File excluded: {}", path.display());
                    return true;
                }
            }
        } else {
            if re.is_match(path.to_str().unwrap()) {
                log_debug!("File excluded: {}", path.display());
                return true;
            }
        }
    }
    log_debug!("File not excluded: {}", path.display());
    false
}

fn get_file_statuses(repo: &Repository) -> Result<Vec<StagedFile>> {
    log_debug!("Getting file statuses");
    let mut staged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for (_index, entry) in statuses.iter().enumerate() {
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
                    let path_obj = Path::new(path);
                    if path_obj.exists() {
                        Some(fs::read_to_string(path_obj)?)
                    } else {
                        None
                    }
                };

            let analyzer = file_analyzers::get_analyzer(path);
            let staged_file = StagedFile {
                path: path.to_string(),
                change_type: change_type.clone(),
                diff: diff.clone(),
                analysis: Vec::new(),
                content: content.clone(),
                content_excluded: should_exclude,
            };
            let analysis = if should_exclude {
                vec!["[Analysis excluded]".to_string()]
            } else {
                analyzer.analyze(path, &staged_file)
            };

            staged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
                analysis,
                content,
                content_excluded: should_exclude,
            });
        }
    }

    log_debug!("Found {} staged files", staged_files.len(),);
    Ok(staged_files)
}

fn get_diff_for_file(repo: &Repository, path: &str) -> Result<String> {
    log_debug!("Getting diff for file: {}", path);
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(path);

    let tree = Some(repo.head()?.peel_to_tree()?);

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

pub async fn get_project_metadata(changed_files: &[String]) -> Result<ProjectMetadata> {
    log_debug!(
        "Getting project metadata for changed files: {:?}",
        changed_files
    );

    let metadata_futures = changed_files.iter().map(|file_path| {
        let file_path = file_path.clone();
        task::spawn(async move {
            let file_name = Path::new(&file_path).file_name().unwrap().to_str().unwrap();
            let analyzer: Box<dyn FileAnalyzer + Send + Sync> =
                file_analyzers::get_analyzer(file_name);

            log_debug!("Analyzing file: {}", file_path);

            if !should_exclude_file(&file_path) {
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    let metadata = analyzer.extract_metadata(file_name, &content);
                    log_debug!("Extracted metadata for {}: {:?}", file_name, metadata);
                    Some(metadata)
                } else {
                    log_debug!("Failed to read file: {}", file_path);
                    None
                }
            } else {
                log_debug!("File excluded: {}", file_path);
                None
            }
        })
    });

    let results = join_all(metadata_futures).await;

    let mut combined_metadata = ProjectMetadata::default();
    let mut any_file_analyzed = false;
    for result in results {
        if let Ok(Some(metadata)) = result {
            log_debug!("Merging metadata: {:?}", metadata);
            merge_metadata(&mut combined_metadata, metadata);
            any_file_analyzed = true;
        }
    }

    log_debug!("Final combined metadata: {:?}", combined_metadata);

    if !any_file_analyzed {
        log_debug!("No files were analyzed!");
        combined_metadata.language = Some("Unknown".to_string());
    } else if combined_metadata.language.is_none() {
        combined_metadata.language = Some("Unknown".to_string());
    }

    Ok(combined_metadata)
}

fn merge_metadata(combined: &mut ProjectMetadata, new: ProjectMetadata) {
    if let Some(new_lang) = new.language {
        match &mut combined.language {
            Some(lang) if !lang.contains(&new_lang) => {
                lang.push_str(", ");
                lang.push_str(&new_lang);
            }
            None => combined.language = Some(new_lang),
            _ => {}
        }
    }
    combined.dependencies.extend(new.dependencies.clone());
    combined.framework = combined.framework.take().or(new.framework);
    combined.version = combined.version.take().or(new.version);
    combined.build_system = combined.build_system.take().or(new.build_system);
    combined.test_framework = combined.test_framework.take().or(new.test_framework);
    combined.plugins.extend(new.plugins);
    combined.dependencies.sort();
    combined.dependencies.dedup();
}

pub fn check_environment() -> Result<()> {
    log_debug!("Checking Git environment");
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_err()
    {
        log_debug!("Git is not installed or not in the PATH");
        return Err(anyhow!("Git is not installed or not in the PATH"));
    }

    log_debug!("Git environment check passed");
    Ok(())
}

pub fn is_inside_work_tree() -> Result<bool> {
    log_debug!("Checking if inside Git work tree");
    match Repository::discover(".") {
        Ok(_) => {
            log_debug!("Inside Git work tree");
            Ok(true)
        }
        Err(_) => {
            log_debug!("Not inside Git work tree");
            Ok(false)
        }
    }
}

pub fn commit(repo_path: &Path, message: &str) -> Result<CommitResult> {
    let repo = Repository::open(repo_path)?;

    // Perform the commit
    let signature = repo.signature()?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent_commit = repo.head()?.peel_to_commit()?;
    let commit_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;

    // Get the branch name
    let branch_name = repo.head()?.shorthand().unwrap_or("HEAD").to_string();

    // Get the short commit hash
    let commit = repo.find_commit(commit_oid)?;
    let short_hash = commit.id().to_string()[..7].to_string();

    // Get diff stats
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;
    let mut new_files = Vec::new();

    let diff = repo.diff_tree_to_tree(Some(&parent_commit.tree()?), Some(&tree), None)?;

    diff.print(git2::DiffFormat::NameStatus, |_, _, line| {
        files_changed += 1;
        if line.origin() == '+' {
            insertions += 1;
        } else if line.origin() == '-' {
            deletions += 1;
        }
        true
    })?;

    // Check for new files
    let statuses = repo.statuses(None)?;
    for entry in statuses.iter() {
        if entry.status().contains(Status::INDEX_NEW) {
            new_files.push((
                entry.path().unwrap().to_string(),
                entry.index_to_workdir().unwrap().new_file().mode(),
            ));
        }
    }

    Ok(CommitResult {
        branch: branch_name,
        commit_hash: short_hash,
        files_changed,
        insertions,
        deletions,
        new_files,
    })
}

pub fn find_and_read_readme(repo_path: &Path) -> Result<Option<String>> {
    log_debug!("Searching for README file in {:?}", repo_path);
    let readme_patterns = ["README.md", "README.txt", "README", "Readme.md"];

    for pattern in readme_patterns.iter() {
        let readme_path = repo_path.join(pattern);
        if readme_path.exists() {
            log_debug!("README file found: {:?}", readme_path);
            let content = fs::read_to_string(readme_path)?;
            return Ok(Some(content));
        }
    }

    log_debug!("No README file found");
    Ok(None)
}
