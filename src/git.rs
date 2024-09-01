use crate::config::Config;
use crate::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use crate::file_analyzers::{self, should_exclude_file, FileAnalyzer};
use crate::log_debug;
use anyhow::{anyhow, Context, Result};
use futures::future::join_all;
use git2::{DiffOptions, FileMode, Repository, Status, StatusOptions, Tree};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tokio::task;

/// Represents a Git repository and provides methods for interacting with it.
pub struct GitRepo {
    repo_path: PathBuf,
}

#[derive(Debug)]
pub struct CommitResult {
    pub branch: String,
    pub commit_hash: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub new_files: Vec<(String, FileMode)>,
}

impl GitRepo {
    /// Creates a new `GitRepo` instance.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - The path to the Git repository.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn new(repo_path: &Path) -> Result<Self> {
        Ok(Self {
            repo_path: repo_path.to_path_buf(),
        })
    }

    /// Open the repository at the stored path
    pub fn open_repo(&self) -> Result<Repository, git2::Error> {
        Repository::open(&self.repo_path)
    }

    /// Retrieves Git information for the repository.
    ///
    /// # Arguments
    ///
    /// * `_config` - The configuration object (currently unused).
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub async fn get_git_info(&self, _config: &Config) -> Result<CommitContext> {
        let repo = self.open_repo()?;
        log_debug!("Getting git info for repo path: {:?}", repo.path());

        let branch = self.get_current_branch()?;
        let recent_commits = self.get_recent_commits(5)?;
        let staged_files = Self::get_file_statuses(&repo)?;

        let changed_files: Vec<String> =
            staged_files.iter().map(|file| file.path.clone()).collect();

        log_debug!("Changed files for metadata extraction: {:?}", changed_files);

        let project_metadata = self.get_project_metadata(&changed_files).await?;

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

    /// Retrieves the current branch name.
    ///
    /// # Returns
    ///
    /// A Result containing the branch name as a String or an error.
    fn get_current_branch(&self) -> Result<String> {
        let repo = self.open_repo()?;
        let head = repo.head()?;
        let branch_name = head.shorthand().unwrap_or("HEAD detached").to_string();
        log_debug!("Current branch: {}", branch_name);
        Ok(branch_name)
    }

    /// Retrieves recent commits.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of recent commits to retrieve.
    ///
    /// # Returns
    ///
    /// A Result containing a Vec of `RecentCommit` objects or an error.
    fn get_recent_commits(&self, count: usize) -> Result<Vec<RecentCommit>> {
        let repo = self.open_repo()?;
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

    /// Retrieves commits between two Git references.
    ///
    /// # Arguments
    ///
    /// * `from` - The starting Git reference.
    /// * `to` - The ending Git reference.
    /// * `callback` - A callback function to process each commit.
    ///
    /// # Returns
    ///
    /// A Result containing a Vec of processed commits or an error.
    pub fn get_commits_between_with_callback<T, F>(
        &self,
        from: &str,
        to: &str,
        mut callback: F,
    ) -> Result<Vec<T>>
    where
        F: FnMut(&RecentCommit) -> Result<T>,
    {
        let repo = self.open_repo()?;
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

    /// Retrieves the status of files in the repository.
    ///
    /// # Returns
    ///
    /// A Result containing a Vec of `StagedFile` objects or an error.
    fn get_file_statuses(repo: &Repository) -> Result<Vec<StagedFile>> {
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
                    Self::get_diff_for_file(repo, path)?
                };

                let content = if should_exclude
                    || change_type != ChangeType::Modified
                    || Self::is_binary_diff(&diff)
                {
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

        log_debug!("Found {} staged files", staged_files.len());
        Ok(staged_files)
    }

    /// Retrieves the diff for a specific file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the file to get the diff for.
    ///
    /// # Returns
    ///
    /// A Result containing the diff as a String or an error.
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

        if Self::is_binary_diff(&diff_string) {
            Ok("[Binary file changed]".to_string())
        } else {
            Ok(diff_string)
        }
    }

    /// Retrieves project metadata for changed files.
    ///
    /// # Arguments
    ///
    /// * `changed_files` - A slice of Strings representing the changed file paths.
    ///
    /// # Returns
    ///
    /// A Result containing the `ProjectMetadata` or an error.
    pub async fn get_project_metadata(&self, changed_files: &[String]) -> Result<ProjectMetadata> {
        log_debug!(
            "Getting project metadata for changed files: {:?}",
            changed_files
        );

        let metadata_futures = changed_files.iter().map(|file_path| {
            let file_path = file_path.clone();
            task::spawn(async move {
                let file_name = Path::new(&file_path)
                    .file_name()
                    .expect("Failed to get file name")
                    .to_str()
                    .expect("Failed to convert file name to string");
                let analyzer: Box<dyn FileAnalyzer + Send + Sync> =
                    file_analyzers::get_analyzer(file_name);

                log_debug!("Analyzing file: {}", file_path);

                if should_exclude_file(&file_path) {
                    log_debug!("File excluded: {}", file_path);
                    None
                } else if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    let metadata = analyzer.extract_metadata(file_name, &content);
                    log_debug!("Extracted metadata for {}: {:?}", file_name, metadata);
                    Some(metadata)
                } else {
                    log_debug!("Failed to read file: {}", file_path);
                    None
                }
            })
        });

        let results = join_all(metadata_futures).await;

        let mut combined_metadata = ProjectMetadata::default();
        let mut any_file_analyzed = false;
        for metadata in results.into_iter().flatten().flatten() {
            log_debug!("Merging metadata: {:?}", metadata);
            combined_metadata.merge(metadata);
            any_file_analyzed = true;
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

    /// Commits changes and verifies the commit.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitResult` or an error.
    pub fn commit_and_verify(&self, message: &str) -> Result<CommitResult> {
        match self.commit(message) {
            Ok(result) => {
                if let Err(e) = self.execute_hook("post-commit") {
                    log_debug!("Post-commit hook failed: {}", e);
                }
                Ok(result)
            }
            Err(e) => {
                log_debug!("Commit failed: {}", e);
                Err(e)
            }
        }
    }

    /// Commits changes to the repository.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitResult` or an error.
    pub fn commit(&self, message: &str) -> Result<CommitResult> {
        let repo = self.open_repo()?;
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

        let branch_name = repo.head()?.shorthand().unwrap_or("HEAD").to_string();
        let commit = repo.find_commit(commit_oid)?;
        let short_hash = commit.id().to_string()[..7].to_string();

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

        let statuses = repo.statuses(None)?;
        for entry in statuses.iter() {
            if entry.status().contains(Status::INDEX_NEW) {
                new_files.push((
                    entry.path().context("Could not get path")?.to_string(),
                    entry
                        .index_to_workdir()
                        .context("Could not get index to workdir")?
                        .new_file()
                        .mode(),
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

    /// Retrieves the README content at a specific commit.
    ///
    /// # Arguments
    ///
    /// * `commit_ish` - A string that resolves to a commit.
    ///
    /// # Returns
    ///
    /// A Result containing an Option<String> with the README content or an error.
    pub fn get_readme_at_commit(&self, commit_ish: &str) -> Result<Option<String>> {
        let repo = self.open_repo()?;
        let obj = repo.revparse_single(commit_ish)?;
        let tree = obj.peel_to_tree()?;

        Self::find_readme_in_tree(&repo, &tree)
            .context("Failed to find and read README at specified commit")
    }

    /// Finds a README file in the given tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - A reference to a Git tree.
    ///
    /// # Returns
    ///
    /// A Result containing an Option<String> with the README content or an error.
    fn find_readme_in_tree(repo: &Repository, tree: &Tree) -> Result<Option<String>> {
        log_debug!("Searching for README file in the repository");

        let readme_patterns = [
            "README.md",
            "README.markdown",
            "README.txt",
            "README",
            "Readme.md",
            "readme.md",
        ];

        for entry in tree {
            let name = entry.name().unwrap_or("");
            if readme_patterns
                .iter()
                .any(|&pattern| name.eq_ignore_ascii_case(pattern))
            {
                let object = entry.to_object(repo)?;
                if let Some(blob) = object.as_blob() {
                    if let Ok(content) = std::str::from_utf8(blob.content()) {
                        log_debug!("README file found: {}", name);
                        return Ok(Some(content.to_string()));
                    }
                }
            }
        }

        log_debug!("No README file found");
        Ok(None)
    }

    /// Executes a Git hook.
    ///
    /// # Arguments
    ///
    /// * `hook_name` - The name of the hook to execute.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error.
    pub fn execute_hook(&self, hook_name: &str) -> Result<()> {
        let repo = self.open_repo()?;
        let hook_path = repo.path().join("hooks").join(hook_name);

        if hook_path.exists() {
            let mut child = Command::new(&hook_path)
                .current_dir(repo.path())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let stdout = child.stdout.take().context("Could not get stdout")?;
            let stderr = child.stderr.take().context("Could not get stderr")?;

            std::thread::spawn(move || {
                io::copy(&mut io::BufReader::new(stdout), &mut io::stdout())
                    .expect("Failed to copy data to stdout");
            });
            std::thread::spawn(move || {
                io::copy(&mut io::BufReader::new(stderr), &mut io::stderr())
                    .expect("Failed to copy data to stderr");
            });

            let status = child.wait()?;

            if !status.success() {
                return Err(anyhow!(
                    "Hook '{}' failed with exit code: {:?}",
                    hook_name,
                    status.code()
                ));
            }
        }

        Ok(())
    }

    /// Checks if the current directory is inside a Git work tree.
    ///
    /// # Returns
    ///
    /// A Result containing a boolean indicating if inside a work tree or an error.
    pub fn is_inside_work_tree() -> Result<bool> {
        log_debug!("Checking if inside Git work tree");
        let repo = Repository::open(env::current_dir()?)?;
        if repo.is_bare() {
            log_debug!("Not inside Git work tree (bare repository)");
            Ok(false)
        } else {
            log_debug!("Inside Git work tree");
            Ok(true)
        }
    }

    fn is_binary_diff(diff: &str) -> bool {
        diff.contains("Binary files")
            || diff.contains("GIT binary patch")
            || diff.contains("[Binary file changed]")
    }
}
