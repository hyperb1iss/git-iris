//! Git operations tools for Rig-based agents
//!
//! This module provides Git operations using Rig's tool system.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::context::{ChangeType, RecentCommit};
use crate::define_tool_error;
use crate::git::StagedFile;

use super::common::{get_current_repo, parameters_schema};

define_tool_error!(GitError);

/// Helper to add a change type if not already present
fn add_change(changes: &mut Vec<&'static str>, change: &'static str) {
    if !changes.contains(&change) {
        changes.push(change);
    }
}

/// Check for function definitions in a line based on language
fn is_function_def(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => {
            line.starts_with("pub fn ")
                || line.starts_with("fn ")
                || line.starts_with("pub async fn ")
                || line.starts_with("async fn ")
        }
        "ts" | "tsx" | "js" | "jsx" => {
            line.starts_with("function ")
                || line.starts_with("async function ")
                || line.contains(" = () =>")
                || line.contains(" = async () =>")
        }
        "py" => line.starts_with("def ") || line.starts_with("async def "),
        "go" => line.starts_with("func "),
        _ => false,
    }
}

/// Check for import statements based on language
fn is_import(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => line.starts_with("use ") || line.starts_with("pub use "),
        "ts" | "tsx" | "js" | "jsx" => line.starts_with("import ") || line.starts_with("export "),
        "py" => line.starts_with("import ") || line.starts_with("from "),
        "go" => line.starts_with("import "),
        _ => false,
    }
}

/// Check for type definitions based on language
fn is_type_def(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => {
            line.starts_with("pub struct ")
                || line.starts_with("struct ")
                || line.starts_with("pub enum ")
                || line.starts_with("enum ")
        }
        "ts" | "tsx" | "js" | "jsx" => {
            line.starts_with("interface ")
                || line.starts_with("type ")
                || line.starts_with("class ")
        }
        "py" => line.starts_with("class "),
        "go" => line.starts_with("type "),
        _ => false,
    }
}

/// Detect semantic change types from diff content
#[allow(clippy::cognitive_complexity)]
fn detect_semantic_changes(diff: &str, path: &str) -> Vec<&'static str> {
    use std::path::Path;

    let mut changes = Vec::new();

    // Get file extension
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    // Only analyze supported languages
    let supported = matches!(
        ext.as_str(),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go"
    );

    if supported {
        // Analyze added lines for patterns
        for line in diff
            .lines()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        {
            let line = line.trim_start_matches('+').trim();

            if is_function_def(line, &ext) {
                add_change(&mut changes, "adds function");
            }
            if is_import(line, &ext) {
                add_change(&mut changes, "modifies imports");
            }
            if is_type_def(line, &ext) {
                add_change(&mut changes, "adds type");
            }
            // Rust-specific: impl blocks
            if ext == "rs" && line.starts_with("impl ") {
                add_change(&mut changes, "adds impl");
            }
        }
    }

    // Check for general change patterns
    let has_deletions = diff
        .lines()
        .any(|l| l.starts_with('-') && !l.starts_with("---"));
    let has_additions = diff
        .lines()
        .any(|l| l.starts_with('+') && !l.starts_with("+++"));

    if has_deletions && has_additions && changes.is_empty() {
        changes.push("refactors code");
    } else if has_deletions && !has_additions {
        changes.push("removes code");
    }

    changes
}

/// Calculate relevance score for a file (0.0 - 1.0)
/// Higher score = more important for commit message
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn calculate_relevance_score(file: &StagedFile) -> (f32, Vec<&'static str>) {
    let mut score: f32 = 0.5; // Base score
    let mut reasons = Vec::new();
    let path = file.path.to_lowercase();

    // Change type scoring
    match file.change_type {
        ChangeType::Added => {
            score += 0.15;
            reasons.push("new file");
        }
        ChangeType::Modified => {
            score += 0.1;
        }
        ChangeType::Deleted => {
            score += 0.05;
            reasons.push("deleted");
        }
    }

    // File type scoring - source code is most important
    if path.ends_with(".rs")
        || path.ends_with(".py")
        || path.ends_with(".ts")
        || path.ends_with(".tsx")
        || path.ends_with(".js")
        || path.ends_with(".jsx")
        || path.ends_with(".go")
        || path.ends_with(".java")
        || path.ends_with(".kt")
        || path.ends_with(".swift")
        || path.ends_with(".c")
        || path.ends_with(".cpp")
        || path.ends_with(".h")
    {
        score += 0.15;
        reasons.push("source code");
    } else if path.ends_with(".toml")
        || path.ends_with(".json")
        || path.ends_with(".yaml")
        || path.ends_with(".yml")
    {
        score += 0.1;
        reasons.push("config");
    } else if path.ends_with(".md") || path.ends_with(".txt") || path.ends_with(".rst") {
        score += 0.02;
        reasons.push("docs");
    }

    // Path-based scoring
    if path.contains("/src/") || path.starts_with("src/") {
        score += 0.1;
        reasons.push("core source");
    }
    if path.contains("/test") || path.contains("_test.") || path.contains(".test.") {
        score -= 0.1;
        reasons.push("test file");
    }
    if path.contains("generated") || path.contains(".lock") || path.contains("package-lock") {
        score -= 0.2;
        reasons.push("generated/lock");
    }
    if path.contains("/vendor/") || path.contains("/node_modules/") {
        score -= 0.3;
        reasons.push("vendored");
    }

    // Diff size scoring (estimate from diff length)
    let diff_lines = file.diff.lines().count();
    if diff_lines > 10 && diff_lines < 200 {
        score += 0.1;
        reasons.push("substantive changes");
    } else if diff_lines >= 200 {
        score += 0.05;
        reasons.push("large diff");
    }

    // Add semantic change detection
    let semantic_changes = detect_semantic_changes(&file.diff, &file.path);
    for change in semantic_changes {
        if !reasons.contains(&change) {
            // Boost score for structural changes
            if change == "adds function" || change == "adds type" || change == "adds impl" {
                score += 0.1;
            }
            reasons.push(change);
        }
    }

    // Clamp to 0.0-1.0
    score = score.clamp(0.0, 1.0);

    (score, reasons)
}

/// Scored file for output
struct ScoredFile<'a> {
    file: &'a StagedFile,
    score: f32,
    reasons: Vec<&'static str>,
}

/// Build the diff output string from scored files
fn format_diff_output(
    scored_files: &[ScoredFile],
    total_files: usize,
    is_filtered: bool,
    include_diffs: bool,
) -> String {
    let mut output = String::new();
    let showing = scored_files.len();

    // Calculate stats
    let additions: usize = scored_files
        .iter()
        .map(|sf| sf.file.diff.lines().filter(|l| l.starts_with('+')).count())
        .sum();
    let deletions: usize = scored_files
        .iter()
        .map(|sf| sf.file.diff.lines().filter(|l| l.starts_with('-')).count())
        .sum();
    let total_lines = additions + deletions;

    // Categorize size
    let (size, guidance) = if is_filtered {
        ("Filtered", "Showing requested files only.")
    } else if total_files <= 3 && total_lines < 100 {
        ("Small", "Focus on all files equally.")
    } else if total_files <= 10 && total_lines < 500 {
        ("Medium", "Prioritize files with >60% relevance.")
    } else {
        (
            "Large",
            "Use files=['path1','path2'] with detail='standard' to analyze specific files.",
        )
    };

    // Header
    let files_info = if is_filtered {
        format!("{showing} of {total_files} files")
    } else {
        format!("{total_files} files")
    };
    output.push_str(&format!(
        "=== CHANGES SUMMARY ===\n{files_info} | +{additions} -{deletions} | Size: {size} ({total_lines} lines)\nGuidance: {guidance}\n\n"
    ));

    // File list
    output.push_str("Files by importance:\n");
    for sf in scored_files {
        let reasons = if sf.reasons.is_empty() {
            String::new()
        } else {
            format!(" ({})", sf.reasons.join(", "))
        };
        output.push_str(&format!(
            "  [{:.0}%] {:?} {}{reasons}\n",
            sf.score * 100.0,
            sf.file.change_type,
            sf.file.path
        ));
    }
    output.push('\n');

    // Diffs or hint
    if include_diffs {
        output.push_str("=== DIFFS ===\n");
        for sf in scored_files {
            output.push_str(&format!(
                "--- {} [{:.0}% relevance]\n",
                sf.file.path,
                sf.score * 100.0
            ));
            output.push_str(&sf.file.diff);
            output.push('\n');
        }
    } else if is_filtered {
        output.push_str("(Use detail='standard' to see full diffs for these files)\n");
    } else {
        output.push_str(
            "(Use detail='standard' with files=['file1','file2'] to see specific diffs)\n",
        );
    }

    output
}

// Git status tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitStatusArgs {
    #[serde(default)]
    pub include_unstaged: bool,
}

impl PortableTool for GitStatus {
    const NAME: &'static str = "git_status";
    type Error = GitError;
    type Args = GitStatusArgs;
    type Output = String;

    fn description(&self) -> String {
        "Get current Git repository status including staged and unstaged files".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitStatusArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;

        let files_info = repo
            .extract_files_info(args.include_unstaged)
            .map_err(GitError::from)?;

        let mut output = String::new();
        output.push_str(&format!("Branch: {}\n", files_info.branch));
        output.push_str(&format!(
            "Files changed: {}\n",
            files_info.staged_files.len()
        ));

        for file in &files_info.staged_files {
            output.push_str(&format!("  {}: {:?}\n", file.path, file.change_type));
        }

        Ok(output)
    }
}

// Git diff tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff;

/// Detail level for diff output
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    /// Summary only: file list with stats and relevance scores, no diffs (default)
    #[default]
    Summary,
    /// Standard: includes full diffs (use with `files` filter for large changesets)
    Standard,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitDiffArgs {
    /// Use "staged" or omit for staged changes, or specify commit/branch
    #[serde(default)]
    pub from: Option<String>,
    /// Target commit/branch (use with from)
    #[serde(default)]
    pub to: Option<String>,
    /// Detail level: "summary" (default) for overview, "standard" for full diffs
    #[serde(default)]
    pub detail: DetailLevel,
    /// Filter to specific files (use with detail="standard" for targeted analysis)
    #[serde(default)]
    pub files: Option<Vec<String>>,
}

impl PortableTool for GitDiff {
    const NAME: &'static str = "git_diff";
    type Error = GitError;
    type Args = GitDiffArgs;
    type Output = String;

    fn description(&self) -> String {
        "Get Git diff for file changes. Returns summary by default (file list with relevance scores). Use detail='standard' with files=['path1','path2'] to get full diffs for specific files. Progressive approach: call once for summary, then again with files filter for important ones.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitDiffArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;

        // Normalize empty strings to None (LLMs often send "" instead of null)
        let from = args.from.filter(|s| !s.is_empty());
        let to = args.to.filter(|s| !s.is_empty());

        // Handle the case where we want staged changes
        // - No args: get staged changes
        // - from="staged": get staged changes
        // - Otherwise: get commit range
        let files = match (from.as_deref(), to.as_deref()) {
            (None | Some("staged"), None) | (Some("staged"), Some("HEAD")) => {
                // Get staged changes
                let files_info = repo.extract_files_info(false).map_err(GitError::from)?;
                files_info.staged_files
            }
            (Some(from), Some(to)) => {
                // Get changes between two commits/branches
                repo.get_commit_range_files(from, to)
                    .map_err(GitError::from)?
            }
            (None, Some(_)) => {
                // Invalid: to without from
                return Err(GitError(
                    "Cannot specify 'to' without 'from'. Use both or neither.".to_string(),
                ));
            }
            (Some(from), None) => {
                // Get changes from a specific commit to HEAD (already handled "staged" above)
                repo.get_commit_range_files(from, "HEAD")
                    .map_err(GitError::from)?
            }
        };

        // Score and sort files by relevance
        let mut scored_files: Vec<ScoredFile> = files
            .iter()
            .map(|file| {
                let (score, reasons) = calculate_relevance_score(file);
                ScoredFile {
                    file,
                    score,
                    reasons,
                }
            })
            .collect();

        // Sort by score descending (most important first)
        scored_files.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Track total before filtering
        let total_files = scored_files.len();

        // Filter to specific files if requested
        let is_filtered = args.files.is_some();
        if let Some(ref filter) = args.files {
            scored_files.retain(|sf| filter.iter().any(|f| sf.file.path.contains(f)));
        }

        // Build output
        let include_diffs = matches!(args.detail, DetailLevel::Standard);
        Ok(format_diff_output(
            &scored_files,
            total_files,
            is_filtered,
            include_diffs,
        ))
    }
}

// Git log tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLog;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitLogArgs {
    #[serde(default)]
    pub count: Option<usize>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

impl PortableTool for GitLog {
    const NAME: &'static str = "git_log";
    type Error = GitError;
    type Args = GitLogArgs;
    type Output = String;

    fn description(&self) -> String {
        "Get Git commit history".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitLogArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;

        if let Some(from) = args.from {
            let to = args.to.unwrap_or_else(|| "HEAD".to_string());
            let commits = repo
                .get_commits_in_range(&from, &to)
                .map_err(GitError::from)?;
            return Ok(format_git_log_output(
                &format!("Commits from {from} to {to}:"),
                &commits,
                true,
            ));
        }

        if args.to.is_some() {
            return Err(GitError::from(anyhow::anyhow!(
                "git_log requires `from` when `to` is provided"
            )));
        }

        let commits = repo
            .get_recent_commits(args.count.unwrap_or(10))
            .map_err(GitError::from)?;

        Ok(format_git_log_output("Recent commits:", &commits, false))
    }
}

fn format_git_log_output(
    header: &str,
    commits: &[RecentCommit],
    include_contributors: bool,
) -> String {
    let mut output = String::new();
    output.push_str(header);
    output.push('\n');

    for commit in commits {
        let title = commit.message.lines().next().unwrap_or_default().trim();
        output.push_str(&format!("{}: {} ({})\n", commit.hash, title, commit.author));
    }

    if include_contributors {
        let contributors: BTreeSet<String> = commits
            .iter()
            .map(|commit| commit.author.trim())
            .filter(|author| !author.is_empty() && !is_bot_author(author))
            .map(ToOwned::to_owned)
            .collect();

        if !contributors.is_empty() {
            output.push_str("\nContributors (excluding bots):\n");
            for contributor in contributors {
                output.push_str(&format!("- {contributor}\n"));
            }
        }
    }

    output
}

fn is_bot_author(author: &str) -> bool {
    let normalized = author.trim().to_ascii_lowercase();

    normalized.contains("[bot]")
        || normalized.contains("dependabot")
        || normalized.contains("renovate")
        || normalized.contains("github-actions")
        || normalized.ends_with(" bot")
        || normalized.ends_with("-bot")
        || normalized == "bot"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitShow;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitShowArgs {
    /// Commit, tag, or branch to inspect.
    pub commit: String,
    /// Optional repository-relative paths to filter the patch.
    #[serde(default)]
    pub files: Option<Vec<PathBuf>>,
    /// Maximum characters to return. Defaults to 20000, clamped to 1000..=50000.
    #[serde(default = "default_git_show_max_output_chars")]
    #[schemars(description = "Maximum characters to return. Values are clamped to 1000..=50000.")]
    pub max_output_chars: usize,
}

fn default_git_show_max_output_chars() -> usize {
    20_000
}

impl PortableTool for GitShow {
    const NAME: &'static str = "git_show";
    type Error = GitError;
    type Args = GitShowArgs;
    type Output = String;

    fn description(&self) -> String {
        "Show a commit message, metadata, stat, and patch for a commit, tag, or branch. Use this after git_log or git_blame when a historical commit's exact changes clarify intent, prior behavior, or regression risk.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitShowArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;
        let repo_root = repo.repo_path();
        let requested = args.commit.trim();
        let commit = resolve_commit(repo_root, requested).map_err(GitError::from)?;
        let files = args
            .files
            .unwrap_or_default()
            .into_iter()
            .map(|file| normalize_repo_relative_filter_path(&file))
            .collect::<Result<Vec<_>>>()
            .map_err(GitError::from)?;
        let max_output_chars = args.max_output_chars.clamp(1_000, 50_000);

        run_git_show(repo_root, requested, &commit, &files, max_output_chars)
            .map_err(GitError::from)
    }
}

fn resolve_commit(repo_root: &Path, commit: &str) -> Result<String> {
    if commit.is_empty() {
        anyhow::bail!("commit must not be empty");
    }
    if commit.starts_with('-') || commit.chars().any(char::is_whitespace) {
        anyhow::bail!("commit must be a commit, tag, or branch name");
    }

    let rev = format!("{commit}^{{commit}}");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", &rev])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("commit not found: {commit}");
    }

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.len() != 40 || !hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        anyhow::bail!("git returned an invalid commit hash for {commit}");
    }

    Ok(hash)
}

fn run_git_show(
    repo_root: &Path,
    requested: &str,
    commit: &str,
    files: &[PathBuf],
    max_output_chars: usize,
) -> Result<String> {
    let mut command = Command::new("git");
    command.args([
        "show",
        "--no-ext-diff",
        "--no-color",
        "--stat",
        "--format=fuller",
        "--patch",
        commit,
    ]);

    if !files.is_empty() {
        command.arg("--");
        command.args(files);
    }

    let output = command.current_dir(repo_root).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git show failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let mut rendered = format!("Git show for {requested} ({commit})\n");
    if !files.is_empty() {
        let file_list = files
            .iter()
            .map(|file| file.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        rendered.push_str(&format!("Filtered paths: {file_list}\n"));
    }
    rendered.push('\n');
    rendered.push_str(String::from_utf8_lossy(&output.stdout).trim_end());

    Ok(truncate_git_show_output(&rendered, max_output_chars))
}

fn truncate_git_show_output(text: &str, max_chars: usize) -> String {
    const SUFFIX: &str = "\n[git_show output truncated]";

    let mut chars = text.chars();
    let mut truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_none() {
        return truncated;
    }

    let suffix_chars = SUFFIX.chars().count();
    let reserved = max_chars.saturating_sub(suffix_chars);
    truncated = text.chars().take(reserved).collect::<String>();
    truncated.push_str(SUFFIX);
    truncated
}

// Git repository info tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepoInfo;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitRepoInfoArgs {}

impl PortableTool for GitRepoInfo {
    const NAME: &'static str = "git_repo_info";
    type Error = GitError;
    type Args = GitRepoInfoArgs;
    type Output = String;

    fn description(&self) -> String {
        "Get general information about the Git repository".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitRepoInfoArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;

        let branch = repo.get_current_branch().map_err(GitError::from)?;
        let remote_url = repo.get_remote_url().unwrap_or("None").to_string();

        let mut output = String::new();
        output.push_str("Repository Information:\n");
        output.push_str(&format!("Current Branch: {branch}\n"));
        output.push_str(&format!("Remote URL: {remote_url}\n"));
        output.push_str(&format!(
            "Repository Path: {}\n",
            repo.repo_path().display()
        ));

        Ok(output)
    }
}

// Git changed files tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitChangedFiles;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitChangedFilesArgs {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

impl PortableTool for GitChangedFiles {
    const NAME: &'static str = "git_changed_files";
    type Error = GitError;
    type Args = GitChangedFilesArgs;
    type Output = String;

    fn description(&self) -> String {
        "Get list of files that have changed between commits or branches".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitChangedFilesArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;

        // Normalize empty strings to None (LLMs often send "" instead of null)
        let from = args.from.filter(|s| !s.is_empty());
        let mut to = args.to.filter(|s| !s.is_empty());

        // Default to HEAD when the caller provides only a starting point.
        if from.is_some() && to.is_none() {
            to = Some("HEAD".to_string());
        }

        let files = match (from, to) {
            (Some(from), Some(to)) => {
                // When both from and to are provided, get files changed between commits/branches
                let range_files = repo
                    .get_commit_range_files(&from, &to)
                    .map_err(GitError::from)?;
                range_files.iter().map(|f| f.path.clone()).collect()
            }
            (None, Some(to)) => {
                // When only to is provided, get files changed in that single commit
                repo.get_file_paths_for_commit(&to)
                    .map_err(GitError::from)?
            }
            (Some(_from), None) => {
                // Invalid: from without to doesn't make sense for file listing
                return Err(GitError(
                    "Cannot specify 'from' without 'to' for file listing".to_string(),
                ));
            }
            (None, None) => {
                // When neither are provided, get staged files
                let files_info = repo.extract_files_info(false).map_err(GitError::from)?;
                files_info.file_paths
            }
        };

        let mut output = String::new();
        output.push_str("Changed files:\n");

        for file in files {
            output.push_str(&format!("  {file}\n"));
        }

        Ok(output)
    }
}

// Git blame tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBlame;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GitBlameArgs {
    /// Repository-relative file path to inspect.
    pub file: PathBuf,
    /// First line to blame, 1-based.
    #[serde(default = "default_start_line")]
    pub start_line: u32,
    /// Last line to blame. Defaults to `start_line`.
    #[serde(default)]
    pub end_line: Option<u32>,
    /// Number of recent commits touching this file to include. Defaults to 3, max 10.
    #[serde(default = "default_recent_commits")]
    pub recent_commits: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlameCommit {
    hash: String,
    author: String,
    date: String,
    summary: String,
}

fn default_start_line() -> u32 {
    1
}

fn default_recent_commits() -> usize {
    3
}

impl PortableTool for GitBlame {
    const NAME: &'static str = "git_blame";
    type Error = GitError;
    type Args = GitBlameArgs;
    type Output = String;

    fn description(&self) -> String {
        "Get git blame context for a repository-relative file line range, plus recent commits that touched the file. Use this for history, ownership, and style context before commit messages, PR descriptions, or semantic explanations.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<GitBlameArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitError::from)?;
        let repo_root = repo.repo_path();
        let file = normalize_repo_relative_path(repo_root, &args.file).map_err(GitError::from)?;
        let start_line = args.start_line.max(1);
        let end_line = args.end_line.unwrap_or(start_line).max(start_line);
        let recent_commits = args.recent_commits.clamp(1, 10);

        let code =
            read_line_range(repo_root, &file, start_line, end_line).map_err(GitError::from)?;
        let blame = if code.in_range {
            run_git_blame(repo_root, &file, start_line, end_line).map_err(GitError::from)?
        } else {
            Vec::new()
        };
        let history =
            recent_file_commits(repo_root, &file, recent_commits).map_err(GitError::from)?;

        Ok(format_blame_output(
            &file,
            start_line,
            end_line,
            &code.content,
            &blame,
            &history,
        ))
    }
}

fn normalize_repo_relative_path(repo_root: &Path, path: &Path) -> Result<PathBuf> {
    let normalized = normalize_repo_relative_filter_path(path)?;
    let full_path = repo_root.join(&normalized);
    if !full_path.is_file() {
        anyhow::bail!("file does not exist: {}", normalized.display());
    }

    Ok(normalized)
}

fn normalize_repo_relative_filter_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        anyhow::bail!("file must be a repository-relative path");
    }

    let normalized = PathBuf::from(
        path.to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./"),
    );

    if normalized.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    }) {
        anyhow::bail!("file must be a repository-relative path");
    }

    Ok(normalized)
}

struct LineRangeContent {
    content: String,
    in_range: bool,
}

fn read_line_range(
    repo_root: &Path,
    file: &Path,
    start_line: u32,
    end_line: u32,
) -> Result<LineRangeContent> {
    let content = std::fs::read_to_string(repo_root.join(file))?;
    let start_index = usize::try_from(start_line.saturating_sub(1))?;
    let take_count = usize::try_from(end_line.saturating_sub(start_line) + 1)?;

    let lines = content
        .lines()
        .enumerate()
        .skip(start_index)
        .take(take_count)
        .map(|(index, line)| format!("{:>4} | {}", index + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    if lines.is_empty() {
        return Ok(LineRangeContent {
            content: format!("<line range outside file: {}>", file.display()),
            in_range: false,
        });
    }

    Ok(LineRangeContent {
        content: lines,
        in_range: true,
    })
}

fn run_git_blame(
    repo_root: &Path,
    file: &Path,
    start_line: u32,
    end_line: u32,
) -> Result<Vec<BlameCommit>> {
    let output = Command::new("git")
        .args([
            "blame",
            "-L",
            &format!("{start_line},{end_line}"),
            "--porcelain",
            "--",
            &file.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git blame failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(parse_blame_porcelain(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_blame_porcelain(output: &str) -> Vec<BlameCommit> {
    let mut commits = Vec::new();
    let mut current_index = None;

    for line in output.lines() {
        if let Some(hash) = line
            .split_whitespace()
            .next()
            .filter(|hash| hash.len() >= 40 && hash.chars().all(|ch| ch.is_ascii_hexdigit()))
        {
            let index = commits
                .iter()
                .position(|commit: &BlameCommit| commit.hash == hash)
                .unwrap_or_else(|| {
                    commits.push(BlameCommit {
                        hash: hash.to_string(),
                        author: String::new(),
                        date: String::new(),
                        summary: String::new(),
                    });
                    commits.len() - 1
                });
            current_index = Some(index);
            continue;
        }

        let Some(index) = current_index else {
            continue;
        };
        let Some(commit) = commits.get_mut(index) else {
            continue;
        };

        if let Some(author) = line.strip_prefix("author ") {
            if commit.author.is_empty() {
                commit.author = author.to_string();
            }
        } else if let Some(timestamp) = line.strip_prefix("author-time ") {
            if commit.date.is_empty() {
                commit.date = timestamp
                    .parse::<i64>()
                    .ok()
                    .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
                    .map_or_else(
                        || "unknown date".to_string(),
                        |datetime| datetime.format("%Y-%m-%d").to_string(),
                    );
            }
        } else if let Some(summary) = line.strip_prefix("summary ")
            && commit.summary.is_empty()
        {
            commit.summary = summary.to_string();
        }
    }

    commits
}

fn recent_file_commits(repo_root: &Path, file: &Path, count: usize) -> Result<Vec<BlameCommit>> {
    let output = Command::new("git")
        .args([
            "log",
            "-n",
            &count.to_string(),
            "--format=%H%x1f%an%x1f%ad%x1f%s",
            "--date=short",
            "--",
            &file.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_log_commit)
        .collect())
}

fn parse_log_commit(line: &str) -> Option<BlameCommit> {
    let mut parts = line.split('\x1f');
    Some(BlameCommit {
        hash: parts.next()?.to_string(),
        author: parts.next()?.to_string(),
        date: parts.next()?.to_string(),
        summary: parts.next()?.to_string(),
    })
}

fn format_blame_output(
    file: &Path,
    start_line: u32,
    end_line: u32,
    code: &str,
    blame: &[BlameCommit],
    history: &[BlameCommit],
) -> String {
    let mut output = format!(
        "Git blame for {}:{start_line}-{end_line}\n\n",
        file.display()
    );
    output.push_str("Code:\n");
    output.push_str(code);
    output.push_str("\n\nBlame commits:\n");

    if blame.is_empty() {
        output.push_str("- No blame data found\n");
    } else {
        for commit in blame {
            output.push_str(&format!(
                "- {}: {} ({}, {})\n",
                short_hash(&commit.hash),
                commit.summary,
                commit.author,
                commit.date
            ));
        }
    }

    output.push_str("\nRecent commits touching this file:\n");
    if history.is_empty() {
        output.push_str("- No recent file history found\n");
    } else {
        for commit in history {
            output.push_str(&format!(
                "- {}: {} ({}, {})\n",
                short_hash(&commit.hash),
                commit.summary,
                commit.author,
                commit.date
            ));
        }
    }

    output
}

fn short_hash(hash: &str) -> &str {
    let end = hash
        .char_indices()
        .nth(8)
        .map_or(hash.len(), |(index, _)| index);
    &hash[..end]
}
