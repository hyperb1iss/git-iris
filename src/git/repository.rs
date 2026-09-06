#![allow(clippy::missing_errors_doc)]

use crate::config::Config;
use crate::context::{CommitContext, RecentCommit, StagedFile};
use crate::git::commit::{self, CommitResult};
use crate::git::files::{
    RepoFilesInfo, get_ahead_behind, get_all_tracked_files, get_file_statuses,
    get_unstaged_file_statuses, get_untracked_files,
};
use crate::git::utils::is_inside_work_tree;
use crate::log_debug;
use anyhow::{Context as AnyhowContext, Result, anyhow};
use chrono::{DateTime, Utc};
use git2::{Repository, Tree};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use url::Url;

/// Represents a Git repository and provides methods for interacting with it.
#[derive(Debug)]
pub struct GitRepo {
    repo_path: PathBuf,
    /// Optional temporary directory for cloned repositories
    #[allow(dead_code)] // This field is needed to maintain ownership of temp directories
    temp_dir: Option<TempDir>,
    /// Whether this is a remote repository
    is_remote: bool,
    /// Original remote URL if this is a cloned repository
    remote_url: Option<String>,
}

fn execute_hook_command(
    hook_name: &str,
    hook_path: &Path,
    git_dir: &Path,
    repo_workdir: &Path,
) -> Result<()> {
    log_hook_start(hook_name, hook_path, repo_workdir);
    wait_for_hook(hook_name, hook_command(hook_path, git_dir, repo_workdir))
}

fn log_hook_start(hook_name: &str, hook_path: &Path, repo_workdir: &Path) {
    log_debug!("Executing hook: {}", hook_name);
    log_debug!("Hook path: {:?}", hook_path);
    log_debug!("Repository working directory: {:?}", repo_workdir);
}

fn hook_command(hook_path: &Path, git_dir: &Path, repo_workdir: &Path) -> Command {
    let mut command = Command::new(hook_path);
    command
        .current_dir(repo_workdir)
        .env("GIT_DIR", git_dir)
        .env("GIT_WORK_TREE", repo_workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    log_debug!("Executing hook command: {:?}", command);
    command
}

fn wait_for_hook(hook_name: &str, mut command: Command) -> Result<()> {
    let mut child = command.spawn()?;
    pipe_hook_output(&mut child)?;

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!(
            "Hook '{}' failed with exit code: {:?}",
            hook_name,
            status.code()
        ));
    }

    log_debug!("Hook '{}' executed successfully", hook_name);
    Ok(())
}

fn pipe_hook_output(child: &mut std::process::Child) -> Result<()> {
    let stdout = child.stdout.take().context("Could not get stdout")?;
    let stderr = child.stderr.take().context("Could not get stderr")?;

    std::thread::spawn(move || copy_hook_stream(stdout, std::io::stdout(), "stdout"));
    std::thread::spawn(move || copy_hook_stream(stderr, std::io::stderr(), "stderr"));
    Ok(())
}

fn copy_hook_stream<R, W>(stream: R, mut output: W, name: &str)
where
    R: std::io::Read,
    W: std::io::Write,
{
    if let Err(e) = std::io::copy(&mut std::io::BufReader::new(stream), &mut output) {
        tracing::debug!("Failed to copy hook {name}: {e}");
    }
}

impl GitRepo {
    /// Creates a new `GitRepo` instance from a local path.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - The path to the Git repository.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo_path = Repository::discover(repo_path)
            .ok()
            .and_then(|repo| {
                repo.workdir()
                    .map(Path::to_path_buf)
                    .or_else(|| repo.path().parent().map(Path::to_path_buf))
            })
            .unwrap_or_else(|| repo_path.to_path_buf());

        Ok(Self {
            repo_path,
            temp_dir: None,
            is_remote: false,
            remote_url: None,
        })
    }

    /// Creates a new `GitRepo` instance, handling both local and remote repositories.
    ///
    /// # Arguments
    ///
    /// * `repository_url` - Optional URL for a remote repository.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn new_from_url(repository_url: Option<String>) -> Result<Self> {
        if let Some(url) = repository_url {
            Self::clone_remote_repository(&url)
        } else {
            let current_dir = env::current_dir()?;
            Self::new(&current_dir)
        }
    }

    /// Clones a remote repository and creates a `GitRepo` instance for it.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the remote repository to clone.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn clone_remote_repository(url: &str) -> Result<Self> {
        log_debug!("Cloning remote repository from URL: {}", url);

        // Validate URL
        let _ = Url::parse(url).map_err(|e| anyhow!("Invalid repository URL: {}", e))?;

        // Create a temporary directory for the clone
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();

        log_debug!("Created temporary directory for clone: {:?}", temp_path);

        // Clone the repository into the temporary directory
        let repo = match Repository::clone(url, temp_path) {
            Ok(repo) => repo,
            Err(e) => return Err(anyhow!("Failed to clone repository: {}", e)),
        };

        log_debug!("Successfully cloned repository to {:?}", repo.path());

        Ok(Self {
            repo_path: temp_path.to_path_buf(),
            temp_dir: Some(temp_dir),
            is_remote: true,
            remote_url: Some(url.to_string()),
        })
    }

    /// Open the repository at the stored path
    pub fn open_repo(&self) -> Result<Repository, git2::Error> {
        Repository::open(&self.repo_path)
    }

    /// Returns whether this `GitRepo` instance is working with a remote repository
    #[must_use]
    pub fn is_remote(&self) -> bool {
        self.is_remote
    }

    /// Returns the original remote URL if this is a cloned repository
    #[must_use]
    pub fn get_remote_url(&self) -> Option<&str> {
        self.remote_url.as_deref()
    }

    /// Returns the repository path
    #[must_use]
    pub fn repo_path(&self) -> &PathBuf {
        &self.repo_path
    }

    /// Updates the remote repository by fetching the latest changes
    pub fn update_remote(&self) -> Result<()> {
        if !self.is_remote {
            return Err(anyhow!("Not a remote repository"));
        }

        log_debug!("Updating remote repository");
        let repo = self.open_repo()?;

        // Find the default remote (usually "origin")
        let remotes = repo.remotes()?;
        let remote_name = remotes
            .iter()
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| anyhow!("No remote found"))?;

        let mut remote = repo.find_remote(remote_name)?;
        let fetch_refspec_storage: Vec<String> = remote
            .fetch_refspecs()?
            .iter()
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .map(std::string::ToString::to_string)
            .collect();
        let fetch_refspecs: Vec<&str> = fetch_refspec_storage
            .iter()
            .map(std::string::String::as_str)
            .collect();

        // Fetch using the remote's configured refspecs so non-main primary
        // branches (for example `trunk`) update correctly too.
        remote.fetch(&fetch_refspecs, None, None)?;

        log_debug!("Successfully updated remote repository");
        Ok(())
    }

    /// Retrieves the current branch name.
    ///
    /// # Returns
    ///
    /// A Result containing the branch name as a String or an error.
    pub fn get_current_branch(&self) -> Result<String> {
        let repo = self.open_repo()?;
        let branch_name = match repo.head() {
            Ok(head) if head.is_branch() => head.shorthand().unwrap_or("HEAD").to_string(),
            Ok(_) => "HEAD detached".to_string(),
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => repo
                .find_reference("HEAD")?
                .symbolic_target()?
                .and_then(|target| target.strip_prefix("refs/heads/"))
                .context("Unborn HEAD has no branch name")?
                .to_string(),
            Err(error) => return Err(error.into()),
        };
        log_debug!("Current branch: {}", branch_name);
        Ok(branch_name)
    }

    /// Resolve the best default base ref for branch comparisons.
    ///
    /// Prefers the repository's remote HEAD when available, falls back to common
    /// primary-branch names, and only uses the current branch as a last resort.
    pub fn get_default_base_ref(&self) -> Result<String> {
        let repo = self.open_repo()?;
        let local_branches = collect_branch_names(&repo, git2::BranchType::Local)?;
        let remote_branches = collect_branch_names(&repo, git2::BranchType::Remote)?;

        if let Some(base) = resolve_remote_head_base(&repo, "origin", &local_branches) {
            return Ok(base);
        }

        if let Ok(remotes) = repo.remotes() {
            for remote_name in &remotes {
                let Some(remote_name) = remote_name? else {
                    continue;
                };
                if remote_name == "origin" {
                    continue;
                }
                if let Some(base) = resolve_remote_head_base(&repo, remote_name, &local_branches) {
                    return Ok(base);
                }
            }
        }

        for candidate in ["main", "master", "trunk", "develop", "dev", "default"] {
            if local_branches.contains(candidate) {
                return Ok(candidate.to_string());
            }
        }

        for candidate in [
            "origin/main",
            "origin/master",
            "origin/trunk",
            "origin/develop",
            "origin/dev",
            "origin/default",
        ] {
            if remote_branches.contains(candidate) {
                return Ok(candidate.to_string());
            }
        }

        self.get_current_branch()
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
        if self.is_remote {
            log_debug!("Skipping hook execution for remote repository");
            return Ok(());
        }

        let repo = self.open_repo()?;
        let repo_workdir = repo
            .workdir()
            .context("Repository has no working directory")?;
        // Git resolves core.hooksPath and the shared directory of linked worktrees.
        let output = Command::new("git")
            .current_dir(repo_workdir)
            .env("GIT_DIR", repo.path())
            .env("GIT_WORK_TREE", repo_workdir)
            .args(["rev-parse", "--git-path", &format!("hooks/{hook_name}")])
            .output()
            .context("Failed to resolve Git hook path")?;
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to resolve Git hook path: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let hook_path = repo_workdir.join(
            String::from_utf8(output.stdout)
                .context("Git hook path is not valid UTF-8")?
                .trim_end_matches(['\r', '\n']),
        );

        if !hook_path.exists() {
            log_debug!("Hook '{}' not found at {:?}", hook_name, hook_path);
            return Ok(());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if hook_path.metadata()?.permissions().mode() & 0o111 == 0 {
                log_debug!("Skipping non-executable hook: {:?}", hook_path);
                return Ok(());
            }
        }
        execute_hook_command(hook_name, &hook_path, repo.path(), repo_workdir)
    }

    /// Get the root directory of the current git repository
    pub fn get_repo_root() -> Result<PathBuf> {
        // Check if we're in a git repository
        if !is_inside_work_tree()? {
            return Err(anyhow!(
                "Not in a Git repository. Please run this command from within a Git repository."
            ));
        }

        // Use git rev-parse to find the repository root
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to get repository root: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Convert the output to a path
        let root = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 output from git command")?
            .trim()
            .to_string();

        Ok(PathBuf::from(root))
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
                if let Some(blob) = object.as_blob()
                    && let Ok(content) = std::str::from_utf8(blob.content())
                {
                    log_debug!("README file found: {}", name);
                    return Ok(Some(content.to_string()));
                }
            }
        }

        log_debug!("No README file found");
        Ok(None)
    }

    /// Extract files info without crossing async boundaries
    pub fn extract_files_info(&self, include_unstaged: bool) -> Result<RepoFilesInfo> {
        let repo = self.open_repo()?;

        // Get basic repo info
        let branch = self.get_current_branch()?;
        let recent_commits = self.get_recent_commits(5)?;

        // Get staged and unstaged files
        let mut staged_files = get_file_statuses(&repo)?;
        if include_unstaged {
            let unstaged_files = self.get_unstaged_files()?;
            staged_files.extend(unstaged_files);
            log_debug!("Combined {} files (staged + unstaged)", staged_files.len());
        }

        // Extract file paths for metadata
        let file_paths: Vec<String> = staged_files.iter().map(|file| file.path.clone()).collect();

        Ok(RepoFilesInfo {
            branch,
            recent_commits,
            staged_files,
            file_paths,
        })
    }

    /// Gets unstaged file changes from the repository
    pub fn get_unstaged_files(&self) -> Result<Vec<StagedFile>> {
        let repo = self.open_repo()?;
        get_unstaged_file_statuses(&repo)
    }

    /// Get diff between two refs as a full unified diff string with headers
    ///
    /// Returns a complete diff suitable for parsing, including:
    /// - diff --git headers
    /// - --- and +++ file headers
    /// - @@ hunk headers
    /// - +/- content lines
    pub fn get_ref_diff_full(&self, from: &str, to: &str) -> Result<String> {
        let repo = self.open_repo()?;

        // Resolve the from and to refs
        let from_commit = repo.revparse_single(from)?.peel_to_commit()?;
        let to_commit = repo.revparse_single(to)?.peel_to_commit()?;

        let from_tree = from_commit.tree()?;
        let to_tree = to_commit.tree()?;

        // Get diff between the two trees
        let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;

        // Format as unified diff
        let mut diff_string = String::new();
        diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            // For diff content lines (+/-/context), prefix with origin char
            if matches!(line.origin(), '+' | '-' | ' ') {
                diff_string.push(line.origin());
            }
            // All line types get their content appended
            diff_string.push_str(&String::from_utf8_lossy(line.content()));

            if line.origin() == 'F'
                && !diff_string.contains("diff --git")
                && let Some(new_file) = delta.new_file().path()
            {
                let header = format!("diff --git a/{0} b/{0}\n", new_file.display());
                if !diff_string.ends_with(&header) {
                    diff_string.insert_str(
                        diff_string.rfind("---").unwrap_or(diff_string.len()),
                        &header,
                    );
                }
            }
            true
        })?;

        Ok(diff_string)
    }

    /// Get staged diff as a full unified diff string with headers
    ///
    /// Returns a complete diff suitable for parsing, including:
    /// - diff --git headers
    /// - --- and +++ file headers
    /// - @@ hunk headers
    /// - +/- content lines
    pub fn get_staged_diff_full(&self) -> Result<String> {
        let repo = self.open_repo()?;

        // Get the HEAD tree to diff against
        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        // Get staged changes (index vs HEAD)
        let diff = repo.diff_tree_to_index(Some(&head_tree), None, None)?;

        // Format as unified diff
        let mut diff_string = String::new();
        diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            // Include all line types for a complete diff
            match line.origin() {
                'H' => {
                    // Hunk header - just the content
                    diff_string.push_str(&String::from_utf8_lossy(line.content()));
                }
                'F' => {
                    // File header
                    diff_string.push_str(&String::from_utf8_lossy(line.content()));
                }
                '+' | '-' | ' ' => {
                    diff_string.push(line.origin());
                    diff_string.push_str(&String::from_utf8_lossy(line.content()));
                }
                '>' | '<' | '=' => {
                    // Binary file markers
                    diff_string.push_str(&String::from_utf8_lossy(line.content()));
                }
                _ => {
                    // Any other content (context info, etc.)
                    diff_string.push_str(&String::from_utf8_lossy(line.content()));
                }
            }

            // Add diff --git header before each file if not present
            if line.origin() == 'F'
                && !diff_string.contains("diff --git")
                && let Some(new_file) = delta.new_file().path()
            {
                let header = format!("diff --git a/{0} b/{0}\n", new_file.display());
                if !diff_string.ends_with(&header) {
                    diff_string.insert_str(
                        diff_string.rfind("---").unwrap_or(diff_string.len()),
                        &header,
                    );
                }
            }
            true
        })?;

        Ok(diff_string)
    }

    /// Retrieves project metadata for changed files.
    /// Helper method for creating `CommitContext`
    ///
    /// # Arguments
    ///
    /// * `branch` - Branch name
    /// * `recent_commits` - List of recent commits
    /// * `staged_files` - List of staged files
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    fn create_commit_context(
        &self,
        branch: String,
        recent_commits: Vec<RecentCommit>,
        staged_files: Vec<StagedFile>,
    ) -> Result<CommitContext> {
        // Get user info
        let repo = self.open_repo()?;
        let user_name = repo.config()?.get_string("user.name").unwrap_or_default();
        let user_email = repo.config()?.get_string("user.email").unwrap_or_default();

        // Create and return the context
        Ok(CommitContext::new(
            branch,
            recent_commits,
            staged_files,
            user_name,
            user_email,
        ))
    }

    /// Retrieves Git information for the repository.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub fn get_git_info(&self, _config: &Config) -> Result<CommitContext> {
        // Get data that doesn't cross async boundaries
        let repo = self.open_repo()?;
        log_debug!("Getting git info for repo path: {:?}", repo.path());

        let branch = self.get_current_branch()?;
        let recent_commits = self.get_recent_commits(5)?;
        let staged_files = get_file_statuses(&repo)?;

        // Create and return the context
        self.create_commit_context(branch, recent_commits, staged_files)
    }

    /// Get Git information including unstaged changes
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `include_unstaged` - Whether to include unstaged changes
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub fn get_git_info_with_unstaged(
        &self,
        _config: &Config,
        include_unstaged: bool,
    ) -> Result<CommitContext> {
        log_debug!("Getting git info with unstaged flag: {}", include_unstaged);

        // Extract all git2 data before crossing async boundaries
        let files_info = self.extract_files_info(include_unstaged)?;

        // Create and return the context
        self.create_commit_context(
            files_info.branch,
            files_info.recent_commits,
            files_info.staged_files,
        )
    }

    /// Get Git information for comparing two branches
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `base_branch` - The base branch (e.g., "main")
    /// * `target_branch` - The target branch (e.g., "feature-branch")
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` for the branch comparison or an error.
    pub fn get_git_info_for_branch_diff(
        &self,
        _config: &Config,
        base_branch: &str,
        target_branch: &str,
    ) -> Result<CommitContext> {
        log_debug!(
            "Getting git info for branch diff: {} -> {}",
            base_branch,
            target_branch
        );
        let repo = self.open_repo()?;

        // Extract branch diff info
        let (display_branch, recent_commits, _file_paths) =
            commit::extract_branch_diff_info(&repo, base_branch, target_branch)?;

        // Get the actual file changes
        let branch_files = commit::get_branch_diff_files(&repo, base_branch, target_branch)?;

        // Create and return the context
        self.create_commit_context(display_branch, recent_commits, branch_files)
    }

    /// Get Git information for a commit range (for PR descriptions)
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `from` - The starting Git reference (exclusive)
    /// * `to` - The ending Git reference (inclusive)
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` for the commit range or an error.
    pub fn get_git_info_for_commit_range(
        &self,
        _config: &Config,
        from: &str,
        to: &str,
    ) -> Result<CommitContext> {
        log_debug!("Getting git info for commit range: {} -> {}", from, to);
        let repo = self.open_repo()?;

        // Extract commit range info
        let (display_range, recent_commits, _file_paths) =
            commit::extract_commit_range_info(&repo, from, to)?;

        // Get the actual file changes
        let range_files = commit::get_commit_range_files(&repo, from, to)?;

        // Create and return the context
        self.create_commit_context(display_range, recent_commits, range_files)
    }

    /// Get commits for PR description between two references
    pub fn get_commits_for_pr(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        commit::get_commits_for_pr(&repo, from, to)
    }

    /// Get commits between two references with author metadata.
    pub fn get_commits_in_range(&self, from: &str, to: &str) -> Result<Vec<RecentCommit>> {
        let repo = self.open_repo()?;
        let mut commits =
            commit::get_commits_between_with_callback(
                &repo,
                from,
                to,
                |commit| Ok(commit.clone()),
            )?;
        commits.reverse();
        Ok(commits)
    }

    /// Get files changed in a commit range  
    pub fn get_commit_range_files(&self, from: &str, to: &str) -> Result<Vec<StagedFile>> {
        let repo = self.open_repo()?;
        commit::get_commit_range_files(&repo, from, to)
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
    pub fn get_recent_commits(&self, count: usize) -> Result<Vec<RecentCommit>> {
        let repo = self.open_repo()?;
        log_debug!("Fetching {} recent commits", count);
        let mut revwalk = repo.revwalk()?;
        match repo.head() {
            Ok(_) => revwalk.push_head()?,
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        }

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
                    timestamp: DateTime::<Utc>::from_timestamp(commit.time().seconds(), 0)
                        .map_or_else(
                            || commit.time().seconds().to_string(),
                            |timestamp| timestamp.to_rfc3339(),
                        ),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        log_debug!("Retrieved {} recent commits", commits.len());
        Ok(commits)
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
        if self.is_remote {
            return Err(anyhow!(
                "Cannot commit to a remote repository in read-only mode"
            ));
        }

        let repo = self.open_repo()?;
        match commit::commit(&repo, message, self.is_remote) {
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

    /// Get Git information for a specific commit
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `commit_id` - The ID of the commit to analyze
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub fn get_git_info_for_commit(
        &self,
        _config: &Config,
        commit_id: &str,
    ) -> Result<CommitContext> {
        log_debug!("Getting git info for commit: {}", commit_id);
        let repo = self.open_repo()?;

        // Get branch name
        let branch = self.get_current_branch()?;

        // Extract commit info
        let commit_info = commit::extract_commit_info(&repo, commit_id, &branch)?;

        // Get the files from commit
        let commit_files = commit::get_commit_files(&repo, commit_id)?;

        // Create and return the context
        self.create_commit_context(commit_info.branch, vec![commit_info.commit], commit_files)
    }

    /// Get the commit date for a reference
    pub fn get_commit_date(&self, commit_ish: &str) -> Result<String> {
        let repo = self.open_repo()?;
        commit::get_commit_date(&repo, commit_ish)
    }

    /// Get commits between two references with a callback
    pub fn get_commits_between_with_callback<T, F>(
        &self,
        from: &str,
        to: &str,
        callback: F,
    ) -> Result<Vec<T>>
    where
        F: FnMut(&RecentCommit) -> Result<T>,
    {
        let repo = self.open_repo()?;
        commit::get_commits_between_with_callback(&repo, from, to, callback)
    }

    /// Commit changes to the repository
    pub fn commit(&self, message: &str) -> Result<CommitResult> {
        let repo = self.open_repo()?;
        commit::commit(&repo, message, self.is_remote)
    }

    /// Amend the previous commit with staged changes and a new message
    pub fn amend_commit(&self, message: &str) -> Result<CommitResult> {
        let repo = self.open_repo()?;
        commit::amend_commit(&repo, message, self.is_remote)
    }

    /// Get the message of the HEAD commit
    pub fn get_head_commit_message(&self) -> Result<String> {
        let repo = self.open_repo()?;
        commit::get_head_commit_message(&repo)
    }

    /// Check if inside a working tree
    pub fn is_inside_work_tree() -> Result<bool> {
        is_inside_work_tree()
    }

    /// Get the files changed in a specific commit
    pub fn get_commit_files(&self, commit_id: &str) -> Result<Vec<StagedFile>> {
        let repo = self.open_repo()?;
        commit::get_commit_files(&repo, commit_id)
    }

    /// Get just the file paths for a specific commit
    pub fn get_file_paths_for_commit(&self, commit_id: &str) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        commit::get_file_paths_for_commit(&repo, commit_id)
    }

    /// Stage a file (add to index)
    pub fn stage_file(&self, path: &Path) -> Result<()> {
        let repo = self.open_repo()?;
        let mut index = repo.index()?;

        // Check if file exists - if not, it might be a deletion
        let full_path = self.repo_path.join(path);
        if full_path.exists() {
            index.add_path(path)?;
        } else {
            // File was deleted, remove from index
            index.remove_path(path)?;
        }

        index.write()?;
        Ok(())
    }

    /// Unstage a file (remove from index, keep working tree changes)
    pub fn unstage_file(&self, path: &Path) -> Result<()> {
        let repo = self.open_repo()?;

        // Get HEAD tree to reset index entry
        let mut index = repo.index()?;
        let head_tree = match repo.head() {
            Ok(head) => Some(head.peel_to_tree()?),
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
            Err(error) => return Err(error.into()),
        };

        // Try to get the entry from HEAD
        if let Some(entry) = head_tree.as_ref().and_then(|tree| tree.get_path(path).ok()) {
            // File exists in HEAD, reset to that state
            let blob = repo.find_blob(entry.id())?;
            #[allow(
                clippy::cast_sign_loss,
                clippy::cast_possible_truncation,
                clippy::as_conversions
            )]
            let file_mode = entry.filemode() as u32;
            #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
            let file_size = blob.content().len() as u32;
            index.add_frombuffer(
                &git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: file_mode,
                    uid: 0,
                    gid: 0,
                    file_size,
                    id: entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.to_string_lossy().as_bytes().to_vec(),
                },
                blob.content(),
            )?;
        } else {
            // File doesn't exist in HEAD (new file), remove from index
            index.remove_path(path)?;
        }

        index.write()?;
        Ok(())
    }

    /// Stage all modified/new/deleted files
    pub fn stage_all(&self) -> Result<()> {
        let repo = self.open_repo()?;
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage all files (reset index to HEAD)
    pub fn unstage_all(&self) -> Result<()> {
        let repo = self.open_repo()?;
        match repo.head() {
            Ok(head) => {
                let head_commit = head.peel_to_commit()?;
                repo.reset(head_commit.as_object(), git2::ResetType::Mixed, None)?;
            }
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {
                let mut index = repo.index()?;
                index.clear()?;
                index.write()?;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    /// Get list of untracked files (new files not in the index)
    pub fn get_untracked_files(&self) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        get_untracked_files(&repo)
    }

    /// Get all tracked files in the repository (from HEAD + index)
    pub fn get_all_tracked_files(&self) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        get_all_tracked_files(&repo)
    }

    /// Get ahead/behind counts relative to upstream tracking branch
    ///
    /// Returns (ahead, behind) tuple, or (0, 0) if no upstream is configured
    #[must_use]
    pub fn get_ahead_behind(&self) -> (usize, usize) {
        let Ok(repo) = self.open_repo() else {
            return (0, 0);
        };
        get_ahead_behind(&repo)
    }
}

fn collect_branch_names(
    repo: &Repository,
    branch_type: git2::BranchType,
) -> Result<HashSet<String>> {
    let mut names = HashSet::new();
    for branch in repo.branches(Some(branch_type))?.flatten() {
        if let Ok(Some(name)) = branch.0.name() {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}

fn resolve_remote_head_base(
    repo: &Repository,
    remote_name: &str,
    local_branches: &HashSet<String>,
) -> Option<String> {
    let reference_name = format!("refs/remotes/{remote_name}/HEAD");
    let Ok(reference) = repo.find_reference(&reference_name) else {
        return None;
    };
    let symbolic_target = reference.symbolic_target().ok()??;
    let remote_ref = symbolic_target.strip_prefix("refs/remotes/")?;

    if let Some((_, local_candidate)) = remote_ref.split_once('/')
        && local_branches.contains(local_candidate)
    {
        return Some(local_candidate.to_string());
    }

    Some(remote_ref.to_string())
}

impl Drop for GitRepo {
    fn drop(&mut self) {
        // The TempDir will be automatically cleaned up when dropped
        if self.is_remote {
            log_debug!("Cleaning up temporary repository at {:?}", self.repo_path);
        }
    }
}
