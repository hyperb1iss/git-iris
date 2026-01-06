//! Main application for Iris Studio
//!
//! Event loop and rendering coordination.

mod agent_tasks;

use anyhow::{Result, anyhow};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::VecDeque;
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::agents::IrisAgentService;
use crate::config::Config;
use crate::git::GitRepo;
use crate::services::GitCommitService;
use crate::types::GeneratedMessage;

use super::components::{DiffHunk, DiffLine, FileDiff, FileGitStatus, parse_diff};
use super::events::{
    AgentResult, ContentPayload, ContentType, SemanticBlameResult, SideEffect, StudioEvent,
    TaskType,
};
use super::history::History;
use super::layout::{LayoutAreas, calculate_layout, get_mode_layout};
use super::reducer::reduce;
use super::render::{
    render_changelog_panel, render_commit_panel, render_companion_status_bar, render_explore_panel,
    render_modal, render_pr_panel, render_release_notes_panel, render_review_panel,
};
use super::state::{GitStatus, IrisStatus, Mode, Notification, PanelId, StudioState};
use super::theme;

// ═══════════════════════════════════════════════════════════════════════════════
// Async Task Results
// ═══════════════════════════════════════════════════════════════════════════════

/// Result from an async Iris task
pub enum IrisTaskResult {
    /// Generated commit messages
    CommitMessages(Vec<GeneratedMessage>),
    /// Generated code review (markdown)
    ReviewContent(String),
    /// Generated PR description (markdown)
    PRContent(String),
    /// Generated changelog (markdown)
    ChangelogContent(String),
    /// Generated release notes (markdown)
    ReleaseNotesContent(String),
    /// Chat response from Iris
    ChatResponse(String),
    /// Chat-triggered update to current content
    ChatUpdate(ChatUpdateType),
    /// Tool call status update (for streaming tool calls to chat)
    ToolStatus { tool_name: String, message: String },
    /// Streaming text chunk received
    StreamingChunk {
        task_type: TaskType,
        chunk: String,
        aggregated: String,
    },
    /// Streaming completed
    StreamingComplete { task_type: TaskType },
    /// Semantic blame result
    SemanticBlame(SemanticBlameResult),
    /// Dynamic status message from fast model
    StatusMessage(crate::agents::StatusMessage),
    /// Completion message from fast model (replaces generic completion)
    CompletionMessage(String),
    /// Error from the task (includes which task failed)
    Error { task_type: TaskType, error: String },
    /// File log loaded for explore mode
    FileLogLoaded {
        file: std::path::PathBuf,
        entries: Vec<crate::studio::state::FileLogEntry>,
    },
    /// Global commit log loaded
    GlobalLogLoaded {
        entries: Vec<crate::studio::state::FileLogEntry>,
    },
    /// Git status loaded (async initialization)
    GitStatusLoaded(Box<GitStatusData>),
    /// Companion service initialized (async)
    CompanionReady(Box<CompanionInitData>),
}

/// Data from async git status loading
#[derive(Debug, Clone)]
pub struct GitStatusData {
    pub branch: String,
    pub staged_files: Vec<std::path::PathBuf>,
    pub modified_files: Vec<std::path::PathBuf>,
    pub untracked_files: Vec<std::path::PathBuf>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
    pub staged_diff: Option<String>,
}

/// Data from async companion initialization
pub struct CompanionInitData {
    /// The companion service
    pub service: crate::companion::CompanionService,
    /// Display data for the UI
    pub display: super::state::CompanionSessionDisplay,
}

/// Type of content update triggered by chat
#[derive(Debug, Clone)]
pub enum ChatUpdateType {
    /// Update commit message
    CommitMessage(GeneratedMessage),
    /// Update PR description
    PRDescription(String),
    /// Update review content
    Review(String),
}

// ═══════════════════════════════════════════════════════════════════════════════
// Studio Application
// ═══════════════════════════════════════════════════════════════════════════════

/// Main Iris Studio application
pub struct StudioApp {
    /// Application state
    pub state: StudioState,
    /// History for all content, chat, and events
    pub history: History,
    /// Event queue for processing
    event_queue: VecDeque<StudioEvent>,
    /// Git commit service for operations
    commit_service: Option<Arc<GitCommitService>>,
    /// Iris agent service for AI operations
    agent_service: Option<Arc<IrisAgentService>>,
    /// Channel receiver for async Iris results
    iris_result_rx: mpsc::UnboundedReceiver<IrisTaskResult>,
    /// Channel sender for async Iris results (kept for spawning tasks)
    iris_result_tx: mpsc::UnboundedSender<IrisTaskResult>,
    /// Last calculated layout for mouse hit testing
    last_layout: Option<LayoutAreas>,
    /// Whether an explicit initial mode was set
    explicit_mode_set: bool,
    /// Last mouse click info for double-click detection (time, x, y)
    last_click: Option<(std::time::Instant, u16, u16)>,
    /// Drag selection start info (panel, line number) for code view selection
    drag_start: Option<(PanelId, usize)>,
    /// Background task handles to abort on exit
    background_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl StudioApp {
    /// Create a new Studio application
    pub fn new(
        config: Config,
        repo: Option<Arc<GitRepo>>,
        commit_service: Option<Arc<GitCommitService>>,
        agent_service: Option<Arc<IrisAgentService>>,
    ) -> Self {
        // Build history with repo context if available
        let history = if let Some(ref r) = repo {
            let repo_path = r.repo_path().clone();
            let branch = r.get_current_branch().ok();
            History::with_repo(repo_path, branch)
        } else {
            History::new()
        };

        let state = StudioState::new(config, repo);
        let (iris_result_tx, iris_result_rx) = mpsc::unbounded_channel();

        Self {
            state,
            history,
            event_queue: VecDeque::new(),
            commit_service,
            agent_service,
            iris_result_rx,
            iris_result_tx,
            last_layout: None,
            explicit_mode_set: false,
            last_click: None,
            drag_start: None,
            background_tasks: Vec::new(),
        }
    }

    /// Set explicit initial mode
    pub fn set_initial_mode(&mut self, mode: Mode) {
        self.state.switch_mode(mode);
        self.explicit_mode_set = true;
    }

    /// Push an event to the queue
    fn push_event(&mut self, event: StudioEvent) {
        self.event_queue.push_back(event);
    }

    /// Process all queued events through the reducer
    fn process_events(&mut self) -> Option<ExitResult> {
        while let Some(event) = self.event_queue.pop_front() {
            // Run through reducer which mutates state and returns effects
            let effects = reduce(&mut self.state, event, &mut self.history);

            // Execute side effects
            if let Some(result) = self.execute_effects(effects) {
                return Some(result);
            }
        }
        None
    }

    /// Execute side effects from reducer
    fn execute_effects(&mut self, effects: Vec<SideEffect>) -> Option<ExitResult> {
        use super::events::{AgentTask, DataType};

        for effect in effects {
            match effect {
                SideEffect::Quit => return Some(ExitResult::Quit),

                SideEffect::ExecuteCommit { message } => {
                    return Some(self.perform_commit(&message));
                }

                SideEffect::ExecuteAmend { message } => {
                    return Some(self.perform_amend(&message));
                }

                SideEffect::Redraw => {
                    self.state.mark_dirty();
                }

                SideEffect::RefreshGitStatus => {
                    let _ = self.refresh_git_status();
                }

                SideEffect::GitStage(path) => {
                    self.stage_file(&path.to_string_lossy());
                }

                SideEffect::GitUnstage(path) => {
                    self.unstage_file(&path.to_string_lossy());
                }

                SideEffect::GitStageAll => {
                    self.stage_all();
                }

                SideEffect::GitUnstageAll => {
                    self.unstage_all();
                }

                SideEffect::SaveSettings => {
                    self.save_settings();
                }

                SideEffect::CopyToClipboard(text) => match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        if let Err(e) = clipboard.set_text(&text) {
                            self.state
                                .notify(Notification::error(format!("Failed to copy: {e}")));
                        } else {
                            self.state
                                .notify(Notification::success("Copied to clipboard"));
                        }
                    }
                    Err(e) => {
                        self.state
                            .notify(Notification::error(format!("Clipboard unavailable: {e}")));
                    }
                },

                SideEffect::ShowNotification {
                    level,
                    message,
                    duration_ms: _,
                } => {
                    let notif = match level {
                        super::events::NotificationLevel::Info => Notification::info(&message),
                        super::events::NotificationLevel::Success => {
                            Notification::success(&message)
                        }
                        super::events::NotificationLevel::Warning => {
                            Notification::warning(&message)
                        }
                        super::events::NotificationLevel::Error => Notification::error(&message),
                    };
                    self.state.notify(notif);
                }

                SideEffect::SpawnAgent { task } => {
                    // Status messages are now spawned inside each spawn_*_generation method
                    match task {
                        AgentTask::Commit {
                            instructions,
                            preset,
                            use_gitmoji,
                            amend,
                        } => {
                            self.spawn_commit_generation(instructions, preset, use_gitmoji, amend);
                        }
                        AgentTask::Review { from_ref, to_ref } => {
                            self.spawn_review_generation(from_ref, to_ref);
                        }
                        AgentTask::PR {
                            base_branch,
                            to_ref,
                        } => {
                            self.spawn_pr_generation(base_branch, &to_ref);
                        }
                        AgentTask::Changelog { from_ref, to_ref } => {
                            self.spawn_changelog_generation(from_ref, to_ref);
                        }
                        AgentTask::ReleaseNotes { from_ref, to_ref } => {
                            self.spawn_release_notes_generation(from_ref, to_ref);
                        }
                        AgentTask::Chat { message, context } => {
                            self.spawn_chat_query(message, context);
                        }
                        AgentTask::SemanticBlame { blame_info } => {
                            self.spawn_semantic_blame(blame_info);
                        }
                    }
                }

                SideEffect::GatherBlameAndSpawnAgent {
                    file,
                    start_line,
                    end_line,
                } => {
                    self.gather_blame_and_spawn(&file, start_line, end_line);
                }

                SideEffect::LoadData {
                    data_type,
                    from_ref,
                    to_ref,
                } => {
                    // Trigger data refresh for the mode
                    match data_type {
                        DataType::GitStatus | DataType::CommitDiff => {
                            let _ = self.refresh_git_status();
                        }
                        DataType::ReviewDiff => {
                            self.update_review_data(from_ref, to_ref);
                        }
                        DataType::PRDiff => {
                            self.update_pr_data(from_ref, to_ref);
                        }
                        DataType::ChangelogCommits => {
                            self.update_changelog_data(from_ref, to_ref);
                        }
                        DataType::ReleaseNotesCommits => {
                            self.update_release_notes_data(from_ref, to_ref);
                        }
                        DataType::ExploreFiles => {
                            self.update_explore_file_tree();
                        }
                    }
                }

                SideEffect::LoadFileLog(path) => {
                    self.load_file_log(&path);
                }

                SideEffect::LoadGlobalLog => {
                    self.load_global_log();
                }
            }
        }
        None
    }

    /// Update git status from repository
    pub fn refresh_git_status(&mut self) -> Result<()> {
        if let Some(repo) = &self.state.repo {
            // Get file info which includes staged files
            let files_info = repo.extract_files_info(false).ok();
            let unstaged = repo.get_unstaged_files().ok();

            let staged_files: Vec<std::path::PathBuf> = files_info
                .as_ref()
                .map(|f| {
                    f.staged_files
                        .iter()
                        .map(|s| s.path.clone().into())
                        .collect()
                })
                .unwrap_or_default();

            let modified_files: Vec<std::path::PathBuf> = unstaged
                .as_ref()
                .map(|f| f.iter().map(|s| s.path.clone().into()).collect())
                .unwrap_or_default();

            // Get untracked files
            let untracked_files: Vec<std::path::PathBuf> = repo
                .get_untracked_files()
                .unwrap_or_default()
                .into_iter()
                .map(std::path::PathBuf::from)
                .collect();

            // Get ahead/behind counts
            let (commits_ahead, commits_behind) = repo.get_ahead_behind();

            let status = GitStatus {
                branch: repo.get_current_branch().unwrap_or_default(),
                staged_count: staged_files.len(),
                staged_files,
                modified_count: modified_files.len(),
                modified_files,
                untracked_count: untracked_files.len(),
                untracked_files,
                commits_ahead,
                commits_behind,
            };
            self.state.git_status = status;

            // Update file trees for components (explore tree is lazy-loaded on mode switch)
            self.update_commit_file_tree();
            self.update_review_file_tree();

            // Load diffs into diff view
            self.load_staged_diffs(files_info.as_ref());

            // Sync initial file selection with diff view
            if let Some(path) = self.state.modes.commit.file_tree.selected_path() {
                self.state.modes.commit.diff_view.select_file_by_path(&path);
            }
        }
        Ok(())
    }

    /// Load staged file diffs into the diff view component
    fn load_staged_diffs(&mut self, files_info: Option<&crate::git::RepoFilesInfo>) {
        let Some(info) = files_info else { return };
        let Some(repo) = &self.state.repo else { return };

        // Get a proper unified diff with all headers using git
        if let Ok(diff_text) = repo.get_staged_diff_full() {
            let diffs = parse_diff(&diff_text);
            self.state.modes.commit.diff_view.set_diffs(diffs);
        } else {
            // Fallback: Build synthetic diff from file info
            let mut diffs = Vec::new();
            for f in &info.staged_files {
                let mut file_diff = FileDiff::new(&f.path);
                file_diff.is_new = matches!(f.change_type, crate::context::ChangeType::Added);
                file_diff.is_deleted = matches!(f.change_type, crate::context::ChangeType::Deleted);

                // Create a synthetic hunk from the diff lines
                if !f.diff.is_empty() && f.diff != "[Content excluded]" {
                    let hunk = DiffHunk {
                        header: "@@ Changes @@".to_string(),
                        lines: f
                            .diff
                            .lines()
                            .enumerate()
                            .map(|(i, line)| {
                                let content = line.strip_prefix(['+', '-', ' ']).unwrap_or(line);
                                if line.starts_with('+') {
                                    DiffLine::added(content, i + 1)
                                } else if line.starts_with('-') {
                                    DiffLine::removed(content, i + 1)
                                } else {
                                    DiffLine::context(content, i + 1, i + 1)
                                }
                            })
                            .collect(),
                        old_start: 1,
                        old_count: 0,
                        new_start: 1,
                        new_count: 0,
                    };
                    file_diff.hunks.push(hunk);
                }
                diffs.push(file_diff);
            }
            self.state.modes.commit.diff_view.set_diffs(diffs);
        }
    }

    /// Update explore mode file tree from repository
    fn update_explore_file_tree(&mut self) {
        // Get all tracked files from the repository
        let Some(repo) = &self.state.repo else { return };
        let all_files: Vec<std::path::PathBuf> = match repo.get_all_tracked_files() {
            Ok(files) => files.into_iter().map(std::path::PathBuf::from).collect(),
            Err(e) => {
                eprintln!("Failed to get tracked files: {}", e);
                return;
            }
        };

        // Build status lookup from git status
        let mut statuses = Vec::new();
        for path in &self.state.git_status.staged_files {
            statuses.push((path.clone(), FileGitStatus::Staged));
        }
        for path in &self.state.git_status.modified_files {
            statuses.push((path.clone(), FileGitStatus::Modified));
        }
        for path in &self.state.git_status.untracked_files {
            statuses.push((path.clone(), FileGitStatus::Untracked));
        }

        if !all_files.is_empty() {
            let tree_state = super::components::FileTreeState::from_paths(&all_files, &statuses);
            self.state.modes.explore.file_tree = tree_state;

            // Initialize selected file (content only - file log loads via event system)
            if let Some(entry) = self.state.modes.explore.file_tree.selected_entry()
                && !entry.is_dir
            {
                let path = entry.path.clone();
                self.state.modes.explore.current_file = Some(path.clone());
                // Load file content into code view
                if let Err(e) = self.state.modes.explore.code_view.load_file(&path) {
                    tracing::warn!("Failed to load initial file: {}", e);
                }
                // Store path for deferred file log loading (will be triggered after event loop starts)
                self.state.modes.explore.pending_file_log = Some(path);
            }
        }
    }

    /// Load git log for a specific file (async)
    fn load_file_log(&self, path: &std::path::Path) {
        use crate::studio::state::FileLogEntry;

        let Some(repo) = &self.state.repo else {
            return;
        };

        let tx = self.iris_result_tx.clone();
        let file = path.to_path_buf();
        let repo_path = repo.repo_path().clone();

        tokio::spawn(async move {
            let file_for_result = file.clone();
            let result = tokio::task::spawn_blocking(move || {
                use std::process::Command;

                // Make file path relative to repo root
                let relative_path = file
                    .strip_prefix(&repo_path)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .to_string();

                // Run git log for the specific file with format:
                // %H = full hash, %h = short hash, %s = subject, %an = author, %ar = relative time
                let output = Command::new("git")
                    .args([
                        "-C",
                        repo_path.to_str().unwrap_or("."),
                        "log",
                        "--follow",
                        "--pretty=format:%H|%h|%s|%an|%ar",
                        "--numstat",
                        "-n",
                        "50", // Limit to 50 entries
                        "--",
                        &relative_path,
                    ])
                    .output()?;

                if !output.status.success() {
                    return Ok(Vec::new());
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut entries = Vec::new();
                let mut current_entry: Option<FileLogEntry> = None;

                for line in stdout.lines() {
                    if line.contains('|') && line.len() > 40 {
                        // This is a commit line
                        if let Some(entry) = current_entry.take() {
                            entries.push(entry);
                        }

                        let parts: Vec<&str> = line.splitn(5, '|').collect();
                        if parts.len() >= 5 {
                            current_entry = Some(FileLogEntry {
                                hash: parts[0].to_string(),
                                short_hash: parts[1].to_string(),
                                message: parts[2].to_string(),
                                author: parts[3].to_string(),
                                relative_time: parts[4].to_string(),
                                additions: None,
                                deletions: None,
                            });
                        }
                    } else if let Some(ref mut entry) = current_entry {
                        // This is a numstat line (additions\tdeletions\tfilename)
                        let stat_parts: Vec<&str> = line.split('\t').collect();
                        if stat_parts.len() >= 2 {
                            entry.additions = stat_parts[0].parse().ok();
                            entry.deletions = stat_parts[1].parse().ok();
                        }
                    }
                }

                // Don't forget the last entry
                if let Some(entry) = current_entry {
                    entries.push(entry);
                }

                Ok::<_, std::io::Error>(entries)
            })
            .await;

            match result {
                Ok(Ok(entries)) => {
                    let _ = tx.send(IrisTaskResult::FileLogLoaded {
                        file: file_for_result,
                        entries,
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to load file log: {}", e);
                }
                Err(e) => {
                    tracing::warn!("File log task panicked: {}", e);
                }
            }
        });
    }

    /// Load global commit log (not file-specific)
    fn load_global_log(&self) {
        use crate::studio::state::FileLogEntry;

        let Some(repo) = &self.state.repo else {
            return;
        };

        let tx = self.iris_result_tx.clone();
        let repo_path = repo.repo_path().clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                use std::process::Command;

                // Run git log for the whole repo
                let output = Command::new("git")
                    .args([
                        "-C",
                        repo_path.to_str().unwrap_or("."),
                        "log",
                        "--pretty=format:%H|%h|%s|%an|%ar",
                        "--shortstat",
                        "-n",
                        "100", // Limit to 100 entries
                    ])
                    .output()?;

                if !output.status.success() {
                    return Ok(Vec::new());
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut entries = Vec::new();
                let mut current_entry: Option<FileLogEntry> = None;

                for line in stdout.lines() {
                    if line.contains('|') && line.len() > 40 {
                        // This is a commit line
                        if let Some(entry) = current_entry.take() {
                            entries.push(entry);
                        }

                        let parts: Vec<&str> = line.splitn(5, '|').collect();
                        if parts.len() >= 5 {
                            current_entry = Some(FileLogEntry {
                                hash: parts[0].to_string(),
                                short_hash: parts[1].to_string(),
                                message: parts[2].to_string(),
                                author: parts[3].to_string(),
                                relative_time: parts[4].to_string(),
                                additions: None,
                                deletions: None,
                            });
                        }
                    } else if line.contains("insertion") || line.contains("deletion") {
                        // This is a shortstat line
                        if let Some(ref mut entry) = current_entry {
                            // Parse "N files changed, M insertions(+), K deletions(-)"
                            for part in line.split(',') {
                                let part = part.trim();
                                if part.contains("insertion") {
                                    entry.additions =
                                        part.split_whitespace().next().and_then(|n| n.parse().ok());
                                } else if part.contains("deletion") {
                                    entry.deletions =
                                        part.split_whitespace().next().and_then(|n| n.parse().ok());
                                }
                            }
                        }
                    }
                }

                // Don't forget the last entry
                if let Some(entry) = current_entry {
                    entries.push(entry);
                }

                Ok::<_, std::io::Error>(entries)
            })
            .await;

            match result {
                Ok(Ok(entries)) => {
                    let _ = tx.send(IrisTaskResult::GlobalLogLoaded { entries });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to load global log: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Global log task panicked: {}", e);
                }
            }
        });
    }

    /// Load git status asynchronously (for fast TUI startup)
    fn load_git_status_async(&self) {
        let Some(repo) = &self.state.repo else {
            return;
        };

        let tx = self.iris_result_tx.clone();
        let repo_path = repo.repo_path().clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                use crate::git::GitRepo;

                // Open repo once and gather all data
                let repo = GitRepo::new(&repo_path)?;

                let branch = repo.get_current_branch().unwrap_or_default();
                let files_info = repo.extract_files_info(false).ok();
                let unstaged = repo.get_unstaged_files().ok();
                let untracked = repo.get_untracked_files().unwrap_or_default();
                let (commits_ahead, commits_behind) = repo.get_ahead_behind();
                let staged_diff = repo.get_staged_diff_full().ok();

                let staged_files: Vec<std::path::PathBuf> = files_info
                    .as_ref()
                    .map(|f| {
                        f.staged_files
                            .iter()
                            .map(|s| s.path.clone().into())
                            .collect()
                    })
                    .unwrap_or_default();

                let modified_files: Vec<std::path::PathBuf> = unstaged
                    .as_ref()
                    .map(|f| f.iter().map(|s| s.path.clone().into()).collect())
                    .unwrap_or_default();

                let untracked_files: Vec<std::path::PathBuf> = untracked
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect();

                Ok::<_, anyhow::Error>(GitStatusData {
                    branch,
                    staged_files,
                    modified_files,
                    untracked_files,
                    commits_ahead,
                    commits_behind,
                    staged_diff,
                })
            })
            .await;

            match result {
                Ok(Ok(data)) => {
                    let _ = tx.send(IrisTaskResult::GitStatusLoaded(Box::new(data)));
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to load git status: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Git status task panicked: {}", e);
                }
            }
        });
    }

    /// Load companion service asynchronously for fast TUI startup
    fn load_companion_async(&mut self) {
        let Some(repo) = &self.state.repo else {
            return;
        };

        let tx = self.iris_result_tx.clone();
        let repo_path = repo.repo_path().clone();
        let branch = repo
            .get_current_branch()
            .unwrap_or_else(|_| "main".to_string());

        let handle = tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                use super::state::CompanionSessionDisplay;
                use crate::companion::{BranchMemory, CompanionService};

                // Create companion service (this is the slow part - file watcher setup)
                let service = CompanionService::new(repo_path, &branch)?;

                // Load or create branch memory
                let mut branch_mem = service
                    .load_branch_memory(&branch)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| BranchMemory::new(branch.clone()));

                // Get welcome message before recording visit
                let welcome = branch_mem.welcome_message();

                // Record this visit
                branch_mem.record_visit();

                // Save updated branch memory
                if let Err(e) = service.save_branch_memory(&branch_mem) {
                    tracing::warn!("Failed to save branch memory: {}", e);
                }

                let display = CompanionSessionDisplay {
                    watcher_active: service.has_watcher(),
                    welcome_message: welcome.clone(),
                    welcome_shown_at: welcome.map(|_| std::time::Instant::now()),
                    ..Default::default()
                };

                Ok::<_, anyhow::Error>(CompanionInitData { service, display })
            })
            .await;

            match result {
                Ok(Ok(data)) => {
                    let _ = tx.send(IrisTaskResult::CompanionReady(Box::new(data)));
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to initialize companion: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Companion init task panicked: {}", e);
                }
            }
        });

        // Store handle so we can abort on exit
        self.background_tasks.push(handle);
    }

    /// Run the TUI application
    pub fn run(&mut self) -> Result<ExitResult> {
        // Install panic hook to ensure terminal is restored on panic
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            // Try to restore terminal
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            // Print panic info to stderr
            eprintln!("\n\n=== PANIC ===\n{}\n", panic_info);
            original_hook(panic_info);
        }));

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run main loop
        let result = self.main_loop(&mut terminal);

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<ExitResult> {
        // Start async git status loading for fast TUI startup
        self.state.git_status_loading = true;
        self.load_git_status_async();

        // Start async companion initialization (file watcher setup is slow)
        self.load_companion_async();

        // Note: Auto-generation happens in apply_git_status_data() after async load completes

        loop {
            // Process any pending file log load (deferred from initialization)
            if let Some(path) = self.state.modes.explore.pending_file_log.take() {
                self.push_event(StudioEvent::FileLogLoading(path));
            }

            // Check for completed Iris tasks
            self.check_iris_results();

            // Poll companion events (file watcher)
            self.check_companion_events();

            // Process any queued events through reducer
            if let Some(result) = self.process_events() {
                return Ok(result);
            }

            // Render if dirty
            if self.state.check_dirty() {
                terminal.draw(|frame| self.render(frame))?;
            }

            // Poll for events with timeout for animations
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        // Only handle key press events
                        if key.kind == KeyEventKind::Press {
                            // Push to event queue - reducer will handle via existing handlers
                            self.push_event(StudioEvent::KeyPressed(key));
                        }
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                let now = std::time::Instant::now();
                                let is_double_click =
                                    self.last_click.is_some_and(|(time, lx, ly)| {
                                        now.duration_since(time).as_millis() < 400
                                            && mouse.column.abs_diff(lx) <= 2
                                            && mouse.row.abs_diff(ly) <= 1
                                    });

                                // Handle click based on what was clicked
                                if let Some(panel) = self.panel_at(mouse.column, mouse.row) {
                                    // Focus panel if not focused
                                    if self.state.focused_panel != panel {
                                        self.state.focused_panel = panel;
                                        self.state.mark_dirty();
                                    }

                                    // Start drag selection for code view
                                    if let Some(line) =
                                        self.code_view_line_at(panel, mouse.column, mouse.row)
                                    {
                                        self.drag_start = Some((panel, line));
                                        // Clear any existing selection and set cursor
                                        self.update_code_selection(panel, line, line);
                                    } else {
                                        self.drag_start = None;
                                    }

                                    // Handle file tree clicks
                                    self.handle_file_tree_click(
                                        panel,
                                        mouse.column,
                                        mouse.row,
                                        is_double_click,
                                    );
                                }

                                // Update last click for double-click detection
                                self.last_click = Some((now, mouse.column, mouse.row));
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                // Extend selection while dragging
                                if let Some((start_panel, start_line)) = self.drag_start
                                    && let Some(panel) = self.panel_at(mouse.column, mouse.row)
                                    && panel == start_panel
                                    && let Some(current_line) =
                                        self.code_view_line_at(panel, mouse.column, mouse.row)
                                {
                                    let (sel_start, sel_end) = if current_line < start_line {
                                        (current_line, start_line)
                                    } else {
                                        (start_line, current_line)
                                    };
                                    self.update_code_selection(panel, sel_start, sel_end);
                                }
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                // Finalize drag selection
                                self.drag_start = None;
                            }
                            _ => {}
                        }
                        // Push scroll events to queue
                        self.push_event(StudioEvent::Mouse(mouse));
                    }
                    Event::Resize(_, _) => {
                        // Terminal resized, trigger redraw
                        self.state.mark_dirty();
                    }
                    _ => {}
                }
            }

            // Push tick event for animations
            self.push_event(StudioEvent::Tick);
        }
    }

    /// Check for companion events (file watcher)
    /// Convert companion events to `StudioEvents` and push to queue
    fn check_companion_events(&mut self) {
        use crate::companion::CompanionEvent;

        // Collect events first to avoid borrow conflicts
        let events: Vec<_> = {
            let Some(companion) = &mut self.state.companion else {
                return;
            };

            let mut collected = Vec::new();
            while let Some(event) = companion.try_recv_event() {
                collected.push(event);
            }
            collected
        };

        // Now convert and push events
        for event in events {
            let studio_event = match event {
                CompanionEvent::FileCreated(path) => StudioEvent::CompanionFileCreated(path),
                CompanionEvent::FileModified(path) => StudioEvent::CompanionFileModified(path),
                CompanionEvent::FileDeleted(path) => StudioEvent::CompanionFileDeleted(path),
                CompanionEvent::FileRenamed(_old, new) => StudioEvent::CompanionFileModified(new),
                CompanionEvent::GitRefChanged => StudioEvent::CompanionGitRefChanged,
                CompanionEvent::WatcherError(err) => StudioEvent::CompanionWatcherError(err),
            };
            self.push_event(studio_event);
        }
    }

    /// Check for completed Iris task results
    /// Convert async Iris results to events and push to queue
    fn check_iris_results(&mut self) {
        while let Ok(result) = self.iris_result_rx.try_recv() {
            let event = match result {
                IrisTaskResult::CommitMessages(messages) => {
                    // Use completion_message from agent if available, otherwise spawn generation
                    if let Some(msg) = messages.first().and_then(|m| m.completion_message.clone()) {
                        tracing::info!("Using agent completion_message: {:?}", msg);
                        self.state.set_iris_complete(msg);
                    } else {
                        tracing::info!(
                            "No completion_message from agent, spawning generation. First msg: {:?}",
                            messages.first().map(|m| &m.title)
                        );
                        let hint = messages.first().map(|m| m.title.clone());
                        self.spawn_completion_message("commit", hint);
                    }
                    StudioEvent::AgentComplete {
                        task_type: TaskType::Commit,
                        result: AgentResult::CommitMessages(messages),
                    }
                }

                IrisTaskResult::ReviewContent(content) => {
                    // Extract first line as hint
                    let hint = content.lines().next().map(|l| l.chars().take(60).collect());
                    self.spawn_completion_message("review", hint);
                    StudioEvent::AgentComplete {
                        task_type: TaskType::Review,
                        result: AgentResult::ReviewContent(content),
                    }
                }

                IrisTaskResult::PRContent(content) => {
                    let hint = content.lines().next().map(|l| l.chars().take(60).collect());
                    self.spawn_completion_message("pr", hint);
                    StudioEvent::AgentComplete {
                        task_type: TaskType::PR,
                        result: AgentResult::PRContent(content),
                    }
                }

                IrisTaskResult::ChangelogContent(content) => {
                    let hint = content.lines().next().map(|l| l.chars().take(60).collect());
                    self.spawn_completion_message("changelog", hint);
                    StudioEvent::AgentComplete {
                        task_type: TaskType::Changelog,
                        result: AgentResult::ChangelogContent(content),
                    }
                }

                IrisTaskResult::ReleaseNotesContent(content) => {
                    let hint = content.lines().next().map(|l| l.chars().take(60).collect());
                    self.spawn_completion_message("release_notes", hint);
                    StudioEvent::AgentComplete {
                        task_type: TaskType::ReleaseNotes,
                        result: AgentResult::ReleaseNotesContent(content),
                    }
                }

                IrisTaskResult::ChatResponse(response) => {
                    // Chat doesn't need fancy completion message
                    StudioEvent::AgentComplete {
                        task_type: TaskType::Chat,
                        result: AgentResult::ChatResponse(response),
                    }
                }

                IrisTaskResult::ChatUpdate(update) => {
                    let (content_type, content) = match update {
                        ChatUpdateType::CommitMessage(msg) => {
                            (ContentType::CommitMessage, ContentPayload::Commit(msg))
                        }
                        ChatUpdateType::PRDescription(content) => (
                            ContentType::PRDescription,
                            ContentPayload::Markdown(content),
                        ),
                        ChatUpdateType::Review(content) => {
                            (ContentType::CodeReview, ContentPayload::Markdown(content))
                        }
                    };
                    StudioEvent::UpdateContent {
                        content_type,
                        content,
                    }
                }

                IrisTaskResult::SemanticBlame(result) => StudioEvent::AgentComplete {
                    task_type: TaskType::SemanticBlame,
                    result: AgentResult::SemanticBlame(result),
                },

                IrisTaskResult::ToolStatus { tool_name, message } => {
                    // Tool status updates - move current tool to history, set new current
                    let tool_desc = format!("{} - {}", tool_name, message);
                    if let Some(prev) = self.state.chat_state.current_tool.take() {
                        self.state.chat_state.add_tool_to_history(prev);
                    }
                    self.state.chat_state.current_tool = Some(tool_desc);
                    self.state.mark_dirty();
                    continue; // Already handled, skip event push
                }

                IrisTaskResult::StreamingChunk {
                    task_type,
                    chunk,
                    aggregated,
                } => StudioEvent::StreamingChunk {
                    task_type,
                    chunk,
                    aggregated,
                },

                IrisTaskResult::StreamingComplete { task_type } => {
                    StudioEvent::StreamingComplete { task_type }
                }

                IrisTaskResult::StatusMessage(message) => {
                    tracing::info!("Received status message via channel: {:?}", message.message);
                    StudioEvent::StatusMessage(message)
                }

                IrisTaskResult::CompletionMessage(message) => {
                    // Directly update status - no event needed
                    tracing::info!("Received completion message: {:?}", message);
                    self.state.set_iris_complete(message);
                    self.state.mark_dirty();
                    continue; // Already handled, skip event push
                }

                IrisTaskResult::Error { task_type, error } => {
                    StudioEvent::AgentError { task_type, error }
                }

                IrisTaskResult::FileLogLoaded { file, entries } => {
                    StudioEvent::FileLogLoaded { file, entries }
                }

                IrisTaskResult::GlobalLogLoaded { entries } => {
                    StudioEvent::GlobalLogLoaded { entries }
                }

                IrisTaskResult::GitStatusLoaded(data) => {
                    // Apply git status data directly (not through reducer)
                    self.apply_git_status_data(*data);
                    continue; // Already handled
                }

                IrisTaskResult::CompanionReady(data) => {
                    // Apply companion data directly
                    self.state.companion = Some(data.service);
                    self.state.companion_display = data.display;
                    self.state.mark_dirty();
                    tracing::info!("Companion service initialized asynchronously");
                    continue; // Already handled
                }
            };

            self.push_event(event);
        }
    }

    /// Apply git status data from async loading
    fn apply_git_status_data(&mut self, data: GitStatusData) {
        use super::components::diff_view::parse_diff;

        // Update git status
        self.state.git_status = super::state::GitStatus {
            branch: data.branch,
            staged_count: data.staged_files.len(),
            staged_files: data.staged_files,
            modified_count: data.modified_files.len(),
            modified_files: data.modified_files,
            untracked_count: data.untracked_files.len(),
            untracked_files: data.untracked_files,
            commits_ahead: data.commits_ahead,
            commits_behind: data.commits_behind,
        };
        self.state.git_status_loading = false;

        // Update file trees
        self.update_commit_file_tree();
        self.update_review_file_tree();

        // Load diffs from staged diff text
        if let Some(diff_text) = data.staged_diff {
            let diffs = parse_diff(&diff_text);
            self.state.modes.commit.diff_view.set_diffs(diffs);
        }

        // Sync initial file selection with diff view
        if let Some(path) = self.state.modes.commit.file_tree.selected_path() {
            self.state.modes.commit.diff_view.select_file_by_path(&path);
        }

        // If no explicit mode was set, switch to suggested mode
        if !self.explicit_mode_set {
            let suggested = self.state.suggest_initial_mode();
            if suggested != self.state.active_mode {
                self.state.switch_mode(suggested);
            }
        }

        // Auto-generate based on mode
        match self.state.active_mode {
            Mode::Commit => {
                if self.state.git_status.has_staged() {
                    self.auto_generate_commit();
                }
            }
            Mode::Review => {
                self.update_review_data(None, None);
                self.auto_generate_review();
            }
            Mode::PR => {
                self.update_pr_data(None, None);
                self.auto_generate_pr();
            }
            Mode::Changelog => {
                self.update_changelog_data(None, None);
                self.auto_generate_changelog();
            }
            Mode::ReleaseNotes => {
                self.update_release_notes_data(None, None);
                self.auto_generate_release_notes();
            }
            Mode::Explore => {
                self.update_explore_file_tree();
            }
        }

        self.state.mark_dirty();
    }

    /// Spawn fire-and-forget status message generation using the fast model
    ///
    /// This spawns an async task that generates witty status messages while
    /// the user waits for the main agent task to complete. Messages are
    /// sent via the result channel and displayed in the status bar.
    fn spawn_status_messages(&self, task: &super::events::AgentTask) {
        use crate::agents::{StatusContext, StatusMessageGenerator};

        tracing::info!("spawn_status_messages called for task: {:?}", task);

        let Some(agent) = self.agent_service.as_ref() else {
            tracing::warn!("No agent service available for status messages");
            return;
        };

        tracing::info!(
            "Status generator using provider={}, fast_model={}",
            agent.provider(),
            agent.fast_model()
        );

        // Build context from task type
        let (task_type, activity) = match task {
            super::events::AgentTask::Commit { amend, .. } => {
                if *amend {
                    ("commit", "amending previous commit")
                } else {
                    ("commit", "crafting your commit message")
                }
            }
            super::events::AgentTask::Review { .. } => ("review", "analyzing code changes"),
            super::events::AgentTask::PR { .. } => ("pr", "drafting PR description"),
            super::events::AgentTask::Changelog { .. } => ("changelog", "generating changelog"),
            super::events::AgentTask::ReleaseNotes { .. } => {
                ("release_notes", "composing release notes")
            }
            super::events::AgentTask::Chat { .. } => ("chat", "thinking about your question"),
            super::events::AgentTask::SemanticBlame { .. } => {
                ("semantic_blame", "tracing code origins")
            }
        };

        let mut context = StatusContext::new(task_type, activity);

        // Add branch if available
        if let Some(repo) = &self.state.repo
            && let Ok(branch) = repo.get_current_branch()
        {
            context = context.with_branch(branch);
        }

        // Collect actual file names for richer context
        let mut files: Vec<String> = Vec::new();
        for file in &self.state.git_status.staged_files {
            // Extract just the filename, not the full path
            let name = file.file_name().map_or_else(
                || file.to_string_lossy().to_string(),
                |n| n.to_string_lossy().to_string(),
            );
            files.push(name);
        }
        for file in &self.state.git_status.modified_files {
            let name = file.file_name().map_or_else(
                || file.to_string_lossy().to_string(),
                |n| n.to_string_lossy().to_string(),
            );
            if !files.contains(&name) {
                files.push(name);
            }
        }

        let file_count = files.len();
        if file_count > 0 {
            context = context.with_file_count(file_count).with_files(files);
        }

        // Detect if this is a regeneration and extract content hint
        let (is_regen, content_hint) = match task {
            super::events::AgentTask::Commit { .. } => {
                let has_content = !self.state.modes.commit.messages.is_empty();
                let hint = self
                    .state
                    .modes
                    .commit
                    .messages
                    .first()
                    .map(|m| m.title.clone());
                (has_content, hint)
            }
            super::events::AgentTask::Review { .. } => {
                let existing = &self.state.modes.review.review_content;
                let hint = existing
                    .lines()
                    .next()
                    .map(|l| l.chars().take(50).collect());
                (!existing.is_empty(), hint)
            }
            super::events::AgentTask::PR { .. } => {
                let existing = &self.state.modes.pr.pr_content;
                let hint = existing
                    .lines()
                    .next()
                    .map(|l| l.chars().take(50).collect());
                (!existing.is_empty(), hint)
            }
            super::events::AgentTask::Changelog { .. } => {
                let existing = &self.state.modes.changelog.changelog_content;
                (!existing.is_empty(), None)
            }
            super::events::AgentTask::ReleaseNotes { .. } => {
                let existing = &self.state.modes.release_notes.release_notes_content;
                (!existing.is_empty(), None)
            }
            _ => (false, None),
        };
        context = context.with_regeneration(is_regen);
        if let Some(hint) = content_hint {
            context = context.with_content_hint(hint);
        }

        // Fire-and-forget: spawn ONE generation attempt
        let tx = self.iris_result_tx.clone();
        let status_gen =
            StatusMessageGenerator::new(agent.provider(), agent.fast_model(), agent.api_key());

        tokio::spawn(async move {
            tracing::info!("Status message starting for task: {}", context.task_type);
            let start = std::time::Instant::now();
            match tokio::time::timeout(
                std::time::Duration::from_millis(2500),
                status_gen.generate(&context),
            )
            .await
            {
                Ok(msg) => {
                    let elapsed = start.elapsed();
                    tracing::info!(
                        "Status message generated in {:?}: {:?}",
                        elapsed,
                        msg.message
                    );
                    if let Err(e) = tx.send(IrisTaskResult::StatusMessage(msg)) {
                        tracing::error!("Failed to send status message: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Status message timed out after {:?}: {}",
                        start.elapsed(),
                        e
                    );
                }
            }
        });
    }

    /// Spawn completion message generation using the fast model
    /// This generates a clever completion message based on the content that was just generated.
    fn spawn_completion_message(&self, task_type: &str, content_hint: Option<String>) {
        use crate::agents::{StatusContext, StatusMessageGenerator};

        let Some(agent) = self.agent_service.as_ref() else {
            return;
        };

        let mut context = StatusContext::new(task_type, "completed");

        // Add branch if available
        if let Some(repo) = &self.state.repo
            && let Ok(branch) = repo.get_current_branch()
        {
            context = context.with_branch(branch);
        }

        // Add content hint for context
        if let Some(hint) = content_hint {
            context = context.with_content_hint(hint);
        }

        let tx = self.iris_result_tx.clone();
        let status_gen =
            StatusMessageGenerator::new(agent.provider(), agent.fast_model(), agent.api_key());

        tokio::spawn(async move {
            match tokio::time::timeout(
                std::time::Duration::from_millis(2000),
                status_gen.generate_completion(&context),
            )
            .await
            {
                Ok(msg) => {
                    tracing::info!("Completion message generated: {:?}", msg.message);
                    let _ = tx.send(IrisTaskResult::CompletionMessage(msg.message));
                }
                Err(_) => {
                    tracing::warn!("Completion message generation timed out");
                }
            }
        });
    }

    /// Auto-generate commit message on app start
    fn auto_generate_commit(&mut self) {
        // Don't regenerate if we already have messages
        if !self.state.modes.commit.messages.is_empty() {
            return;
        }

        self.state.set_iris_thinking("Analyzing changes...");
        self.state.modes.commit.generating = true;
        let preset = self.state.modes.commit.preset.clone();
        let use_gitmoji = self.state.modes.commit.use_gitmoji;
        let amend = self.state.modes.commit.amend_mode;
        self.spawn_commit_generation(None, preset, use_gitmoji, amend);
    }

    /// Auto-generate code review on mode entry
    fn auto_generate_review(&mut self) {
        // Don't regenerate if we already have content
        if !self.state.modes.review.review_content.is_empty() {
            return;
        }

        // Need diffs to review
        if self.state.modes.review.diff_view.file_paths().is_empty() {
            return;
        }

        self.state.set_iris_thinking("Reviewing code changes...");
        self.state.modes.review.generating = true;
        let from_ref = self.state.modes.review.from_ref.clone();
        let to_ref = self.state.modes.review.to_ref.clone();
        self.spawn_review_generation(from_ref, to_ref);
    }

    /// Auto-generate PR description on mode entry
    fn auto_generate_pr(&mut self) {
        // Don't regenerate if we already have content
        if !self.state.modes.pr.pr_content.is_empty() {
            return;
        }

        // Need commits to describe
        if self.state.modes.pr.commits.is_empty() {
            return;
        }

        self.state.set_iris_thinking("Drafting PR description...");
        self.state.modes.pr.generating = true;
        let base_branch = self.state.modes.pr.base_branch.clone();
        let to_ref = self.state.modes.pr.to_ref.clone();
        self.spawn_pr_generation(base_branch, &to_ref);
    }

    /// Auto-generate changelog on mode entry
    fn auto_generate_changelog(&mut self) {
        // Don't regenerate if we already have content
        if !self.state.modes.changelog.changelog_content.is_empty() {
            return;
        }

        // Need commits to generate from
        if self.state.modes.changelog.commits.is_empty() {
            return;
        }

        let from_ref = self.state.modes.changelog.from_ref.clone();
        let to_ref = self.state.modes.changelog.to_ref.clone();

        self.state.set_iris_thinking("Generating changelog...");
        self.state.modes.changelog.generating = true;
        self.spawn_changelog_generation(from_ref, to_ref);
    }

    /// Auto-generate release notes on mode entry
    fn auto_generate_release_notes(&mut self) {
        // Don't regenerate if we already have content
        if !self
            .state
            .modes
            .release_notes
            .release_notes_content
            .is_empty()
        {
            return;
        }

        // Need commits to generate from
        if self.state.modes.release_notes.commits.is_empty() {
            return;
        }

        let from_ref = self.state.modes.release_notes.from_ref.clone();
        let to_ref = self.state.modes.release_notes.to_ref.clone();

        self.state.set_iris_thinking("Generating release notes...");
        self.state.modes.release_notes.generating = true;
        self.spawn_release_notes_generation(from_ref, to_ref);
    }

    /// Determine which panel contains the given coordinates
    fn panel_at(&self, x: u16, y: u16) -> Option<PanelId> {
        let Some(layout) = &self.last_layout else {
            return None;
        };

        for (i, panel_rect) in layout.panels.iter().enumerate() {
            if x >= panel_rect.x
                && x < panel_rect.x + panel_rect.width
                && y >= panel_rect.y
                && y < panel_rect.y + panel_rect.height
            {
                return match i {
                    0 => Some(PanelId::Left),
                    1 => Some(PanelId::Center),
                    2 => Some(PanelId::Right),
                    _ => None,
                };
            }
        }
        None
    }

    /// Handle mouse click in panels (file tree, code view, etc.)
    fn handle_file_tree_click(&mut self, panel: PanelId, _x: u16, y: u16, is_double_click: bool) {
        let Some(layout) = &self.last_layout else {
            return;
        };

        // Get the panel rect
        let panel_idx = match panel {
            PanelId::Left => 0,
            PanelId::Center => 1,
            PanelId::Right => 2,
        };

        let Some(panel_rect) = layout.panels.get(panel_idx) else {
            return;
        };

        // Calculate row within panel (accounting for border and title)
        // Panel has 1 row border on each side
        let inner_y = y.saturating_sub(panel_rect.y + 1);

        // Determine which component to update based on mode and panel
        match (self.state.active_mode, panel) {
            // ─────────────────────────────────────────────────────────────────
            // Explore Mode
            // ─────────────────────────────────────────────────────────────────
            (Mode::Explore, PanelId::Left) => {
                let file_tree = &mut self.state.modes.explore.file_tree;
                let (changed, is_dir) = file_tree.handle_click(inner_y as usize);

                if is_double_click && is_dir {
                    file_tree.toggle_expand();
                } else if is_double_click && !is_dir {
                    // Double-click on file: load it and focus code view
                    if let Some(path) = file_tree.selected_path() {
                        self.state.modes.explore.current_file = Some(path.clone());
                        if let Err(e) = self.state.modes.explore.code_view.load_file(&path) {
                            self.state.notify(Notification::warning(format!(
                                "Could not load file: {}",
                                e
                            )));
                        }
                        self.state.focused_panel = PanelId::Center;
                    }
                } else if changed && !is_dir {
                    // Single click on file: load it into code view
                    if let Some(path) = file_tree.selected_path() {
                        self.state.modes.explore.current_file = Some(path.clone());
                        if let Err(e) = self.state.modes.explore.code_view.load_file(&path) {
                            self.state.notify(Notification::warning(format!(
                                "Could not load file: {}",
                                e
                            )));
                        }
                    }
                }
                self.state.mark_dirty();
            }
            (Mode::Explore, PanelId::Center) => {
                // Code view: click to select line
                let code_view = &mut self.state.modes.explore.code_view;
                if code_view.select_by_row(inner_y as usize) {
                    self.state.mark_dirty();
                }
            }
            // ─────────────────────────────────────────────────────────────────
            // Commit Mode
            // ─────────────────────────────────────────────────────────────────
            (Mode::Commit, PanelId::Left) => {
                let file_tree = &mut self.state.modes.commit.file_tree;
                let (changed, is_dir) = file_tree.handle_click(inner_y as usize);

                if is_double_click && is_dir {
                    file_tree.toggle_expand();
                } else if is_double_click && !is_dir {
                    // Double-click on file: focus on diff panel
                    if let Some(path) = file_tree.selected_path() {
                        self.state.modes.commit.diff_view.select_file_by_path(&path);
                        self.state.focused_panel = PanelId::Right;
                    }
                } else if changed {
                    // Single click: sync diff view
                    if let Some(path) = file_tree.selected_path() {
                        self.state.modes.commit.diff_view.select_file_by_path(&path);
                    }
                }
                self.state.mark_dirty();
            }
            // ─────────────────────────────────────────────────────────────────
            // Review Mode
            // ─────────────────────────────────────────────────────────────────
            (Mode::Review, PanelId::Left) => {
                let file_tree = &mut self.state.modes.review.file_tree;
                let (changed, is_dir) = file_tree.handle_click(inner_y as usize);

                if is_double_click && is_dir {
                    file_tree.toggle_expand();
                } else if is_double_click && !is_dir {
                    // Double-click on file: focus on diff panel
                    if let Some(path) = file_tree.selected_path() {
                        self.state.modes.review.diff_view.select_file_by_path(&path);
                        self.state.focused_panel = PanelId::Center;
                    }
                } else if changed {
                    // Single click: sync diff view
                    if let Some(path) = file_tree.selected_path() {
                        self.state.modes.review.diff_view.select_file_by_path(&path);
                    }
                }
                self.state.mark_dirty();
            }
            // ─────────────────────────────────────────────────────────────────
            // PR Mode
            // ─────────────────────────────────────────────────────────────────
            (Mode::PR, PanelId::Left) => {
                let file_tree = &mut self.state.modes.pr.file_tree;
                let (changed, is_dir) = file_tree.handle_click(inner_y as usize);

                if is_double_click && is_dir {
                    file_tree.toggle_expand();
                } else if (changed || is_double_click)
                    && let Some(path) = file_tree.selected_path()
                {
                    self.state.modes.pr.diff_view.select_file_by_path(&path);
                }
                self.state.mark_dirty();
            }
            _ => {}
        }
    }

    /// Get the line number at a mouse position in the code view (1-indexed)
    /// Returns None if not in a code view area
    fn code_view_line_at(&self, panel: PanelId, _x: u16, y: u16) -> Option<usize> {
        let layout = self.last_layout.as_ref()?;

        // Get the panel rect
        let panel_idx = match panel {
            PanelId::Left => 0,
            PanelId::Center => 1,
            PanelId::Right => 2,
        };

        let panel_rect = layout.panels.get(panel_idx)?;

        // Calculate row within panel (accounting for border)
        let inner_y = y.saturating_sub(panel_rect.y + 1) as usize;

        // Only handle code view panels based on mode
        match (self.state.active_mode, panel) {
            (Mode::Explore, PanelId::Center) => {
                let code_view = &self.state.modes.explore.code_view;
                let target_line = code_view.scroll_offset() + inner_y + 1;
                if target_line <= code_view.line_count() {
                    Some(target_line)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Update code view selection range (for mouse drag)
    fn update_code_selection(&mut self, panel: PanelId, start: usize, end: usize) {
        if let (Mode::Explore, PanelId::Center) = (self.state.active_mode, panel) {
            // Update code view state
            self.state.modes.explore.code_view.set_selected_line(start);
            if start == end {
                // Single line - set anchor for potential drag extension
                self.state.modes.explore.selection_anchor = Some(start);
                self.state.modes.explore.code_view.clear_selection();
                self.state.modes.explore.selection = None;
            } else {
                // Multi-line selection from drag
                self.state.modes.explore.code_view.set_selection(start, end);
                self.state.modes.explore.selection = Some((start, end));
            }
            // Update current line for semantic blame
            self.state.modes.explore.current_line = start;
            self.state.mark_dirty();
        }
    }

    fn perform_commit(&mut self, message: &str) -> ExitResult {
        if let Some(service) = &self.commit_service {
            match service.perform_commit(message) {
                Ok(result) => {
                    // Record commit in companion
                    self.state
                        .companion_record_commit(result.commit_hash.clone());

                    // Also update branch memory commit count
                    self.update_branch_commit_count(&result.branch);

                    let output = crate::output::format_commit_result(&result, message);
                    ExitResult::Committed(output)
                }
                Err(e) => ExitResult::Error(e.to_string()),
            }
        } else {
            ExitResult::Error("Commit service not available".to_string())
        }
    }

    fn perform_amend(&mut self, message: &str) -> ExitResult {
        if let Some(service) = &self.commit_service {
            match service.perform_amend(message) {
                Ok(result) => {
                    // Record amend in companion (still counts as commit activity)
                    self.state
                        .companion_record_commit(result.commit_hash.clone());

                    let output = crate::output::format_commit_result(&result, message);
                    ExitResult::Amended(output)
                }
                Err(e) => ExitResult::Error(e.to_string()),
            }
        } else {
            ExitResult::Error("Commit service not available".to_string())
        }
    }

    /// Update branch memory commit count
    fn update_branch_commit_count(&self, branch: &str) {
        if let Some(ref companion) = self.state.companion {
            let mut branch_mem = companion
                .load_branch_memory(branch)
                .ok()
                .flatten()
                .unwrap_or_else(|| crate::companion::BranchMemory::new(branch.to_string()));

            branch_mem.record_commit();

            if let Err(e) = companion.save_branch_memory(&branch_mem) {
                tracing::warn!("Failed to save branch memory after commit: {}", e);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Rendering
    // ═══════════════════════════════════════════════════════════════════════════

    fn render(&mut self, frame: &mut Frame) {
        let areas = calculate_layout(frame.area(), self.state.active_mode);

        self.render_header(frame, areas.header);
        self.render_tabs(frame, areas.tabs);
        self.render_panels(frame, &areas);

        // Render companion status bar for explore mode
        if let Some(companion_area) = areas.companion_bar {
            render_companion_status_bar(frame, companion_area, &self.state);
        }

        self.render_status(frame, areas.status);

        // Store layout for mouse hit testing
        self.last_layout = Some(areas);

        // Render modal overlay on top of everything
        if self.state.modal.is_some() {
            render_modal(&self.state, frame, self.state.last_render);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let branch = &self.state.git_status.branch;
        let staged = self.state.git_status.staged_count;
        let modified = self.state.git_status.modified_count;

        // Create gradient title "◆ Iris Studio"
        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled(
            " ◆ ",
            Style::default().fg(theme::accent_primary()),
        ));

        // Gradient text for "Iris Studio"
        let title_text = "Iris Studio";
        #[allow(clippy::cast_precision_loss)]
        for (i, c) in title_text.chars().enumerate() {
            let position = i as f32 / (title_text.len() - 1).max(1) as f32;
            spans.push(Span::styled(
                c.to_string(),
                Style::default()
                    .fg(theme::gradient_purple_cyan(position))
                    .add_modifier(Modifier::BOLD),
            ));
        }

        spans.push(Span::raw(" "));

        // Branch info with git icon
        if !branch.is_empty() {
            spans.push(Span::styled(
                "⎇ ",
                Style::default().fg(theme::text_dim_color()),
            ));
            spans.push(Span::styled(
                format!("{} ", branch),
                Style::default()
                    .fg(theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Staged count
        if staged > 0 {
            spans.push(Span::styled(
                format!("✓{} ", staged),
                Style::default().fg(theme::success_color()),
            ));
        }

        // Modified count
        if modified > 0 {
            spans.push(Span::styled(
                format!("○{} ", modified),
                Style::default().fg(theme::warning_color()),
            ));
        }

        let line = Line::from(spans);
        let header = Paragraph::new(line);
        frame.render_widget(header, area);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();
        spans.push(Span::raw(" "));

        for (idx, mode) in Mode::all().iter().enumerate() {
            let is_active = *mode == self.state.active_mode;
            let is_available = mode.is_available();

            if is_active {
                // Active tab with gradient underline effect
                spans.push(Span::styled(
                    format!(" {} ", mode.shortcut()),
                    Style::default()
                        .fg(theme::accent_primary())
                        .add_modifier(Modifier::BOLD),
                ));
                // Mode name with gradient
                let name = mode.display_name();
                #[allow(clippy::cast_precision_loss)]
                for (i, c) in name.chars().enumerate() {
                    let position = i as f32 / (name.len() - 1).max(1) as f32;
                    spans.push(Span::styled(
                        c.to_string(),
                        Style::default()
                            .fg(theme::gradient_purple_cyan(position))
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                spans.push(Span::raw(" "));
                // Underline with gradient
                spans.push(Span::styled(
                    "━",
                    Style::default().fg(theme::accent_primary()),
                ));
                spans.push(Span::styled(
                    "━",
                    Style::default().fg(theme::gradient_purple_cyan(0.5)),
                ));
                spans.push(Span::styled(
                    "━",
                    Style::default().fg(theme::accent_secondary()),
                ));
            } else if is_available {
                spans.push(Span::styled(
                    format!(" {} ", mode.shortcut()),
                    Style::default().fg(theme::text_muted_color()),
                ));
                spans.push(Span::styled(
                    mode.display_name().to_string(),
                    theme::mode_inactive(),
                ));
            } else {
                spans.push(Span::styled(
                    format!(" {} {} ", mode.shortcut(), mode.display_name()),
                    Style::default().fg(theme::text_muted_color()),
                ));
            }

            // Separator between tabs
            if idx < Mode::all().len() - 1 {
                spans.push(Span::styled(
                    " │ ",
                    Style::default().fg(theme::text_muted_color()),
                ));
            }
        }

        let tabs = Paragraph::new(Line::from(spans));
        frame.render_widget(tabs, area);
    }

    fn render_panels(&mut self, frame: &mut Frame, areas: &LayoutAreas) {
        let layout = get_mode_layout(self.state.active_mode);
        let panel_ids: Vec<_> = layout.panels.iter().map(|c| c.id).collect();
        let panel_areas: Vec<_> = areas.panels.clone();

        for (i, panel_area) in panel_areas.iter().enumerate() {
            if let Some(&panel_id) = panel_ids.get(i) {
                self.render_panel_content(frame, *panel_area, panel_id);
            }
        }
    }

    fn render_panel_content(&mut self, frame: &mut Frame, area: Rect, panel_id: PanelId) {
        match self.state.active_mode {
            Mode::Explore => render_explore_panel(&mut self.state, frame, area, panel_id),
            Mode::Commit => render_commit_panel(&mut self.state, frame, area, panel_id),
            Mode::Review => render_review_panel(&mut self.state, frame, area, panel_id),
            Mode::PR => render_pr_panel(&mut self.state, frame, area, panel_id),
            Mode::Changelog => render_changelog_panel(&mut self.state, frame, area, panel_id),
            Mode::ReleaseNotes => {
                render_release_notes_panel(&mut self.state, frame, area, panel_id);
            }
        }
    }

    /// Update commit mode file tree from git status
    /// Shows either changed files (staged/unstaged) or all tracked files based on toggle
    fn update_commit_file_tree(&mut self) {
        let mut statuses = Vec::new();

        // Build status map for known changed files
        for path in &self.state.git_status.staged_files {
            statuses.push((path.clone(), FileGitStatus::Staged));
        }
        for path in &self.state.git_status.modified_files {
            if !self.state.git_status.staged_files.contains(path) {
                statuses.push((path.clone(), FileGitStatus::Modified));
            }
        }
        for path in &self.state.git_status.untracked_files {
            statuses.push((path.clone(), FileGitStatus::Untracked));
        }

        let all_files: Vec<std::path::PathBuf> = if self.state.modes.commit.show_all_files {
            // Show all tracked files from the repository
            let Some(repo) = &self.state.repo else {
                return;
            };
            match repo.get_all_tracked_files() {
                Ok(files) => files.into_iter().map(std::path::PathBuf::from).collect(),
                Err(e) => {
                    eprintln!("Failed to get tracked files: {}", e);
                    return;
                }
            }
        } else {
            // Show only changed files (staged + modified + untracked)
            let mut files = Vec::new();
            for path in &self.state.git_status.staged_files {
                files.push(path.clone());
            }
            for path in &self.state.git_status.modified_files {
                if !files.contains(path) {
                    files.push(path.clone());
                }
            }
            for path in &self.state.git_status.untracked_files {
                if !files.contains(path) {
                    files.push(path.clone());
                }
            }
            files
        };

        let tree_state = super::components::FileTreeState::from_paths(&all_files, &statuses);
        self.state.modes.commit.file_tree = tree_state;

        // Expand all by default (usually not too many files)
        self.state.modes.commit.file_tree.expand_all();
    }

    /// Update review mode file tree from git status (staged + modified)
    fn update_review_file_tree(&mut self) {
        let mut all_files = Vec::new();
        let mut statuses = Vec::new();

        // Include both staged and modified files for review
        for path in &self.state.git_status.staged_files {
            all_files.push(path.clone());
            statuses.push((path.clone(), FileGitStatus::Staged));
        }
        for path in &self.state.git_status.modified_files {
            if !all_files.contains(path) {
                all_files.push(path.clone());
                statuses.push((path.clone(), FileGitStatus::Modified));
            }
        }

        let tree_state = super::components::FileTreeState::from_paths(&all_files, &statuses);
        self.state.modes.review.file_tree = tree_state;
        self.state.modes.review.file_tree.expand_all();

        // Also load diffs for review mode
        self.load_review_diffs();
    }

    /// Load diffs into review mode diff view
    fn load_review_diffs(&mut self) {
        let Some(repo) = &self.state.repo else { return };

        // Get staged diff first, then unstaged
        if let Ok(diff_text) = repo.get_staged_diff_full() {
            let diffs = parse_diff(&diff_text);
            self.state.modes.review.diff_view.set_diffs(diffs);
        }

        // Sync initial file selection
        if let Some(path) = self.state.modes.review.file_tree.selected_path() {
            self.state.modes.review.diff_view.select_file_by_path(&path);
        }
    }

    /// Update PR mode data - load commits and diff between refs
    pub fn update_pr_data(&mut self, from_ref: Option<String>, to_ref: Option<String>) {
        use super::state::PrCommit;

        // Clone the Arc to avoid borrow conflicts with self.state mutations
        let Some(repo) = self.state.repo.clone() else {
            return;
        };

        // Use provided refs or fall back to state
        let base = from_ref.unwrap_or_else(|| self.state.modes.pr.base_branch.clone());
        let to = to_ref.unwrap_or_else(|| self.state.modes.pr.to_ref.clone());

        // Load commits between the refs
        match repo.get_commits_between_with_callback(&base, &to, |commit| {
            Ok(PrCommit {
                hash: commit.hash[..7.min(commit.hash.len())].to_string(),
                message: commit.message.lines().next().unwrap_or("").to_string(),
                author: commit.author.clone(),
            })
        }) {
            Ok(commits) => {
                self.state.modes.pr.commits = commits;
                self.state.modes.pr.selected_commit = 0;
                self.state.modes.pr.commit_scroll = 0;
            }
            Err(e) => {
                self.state.notify(Notification::warning(format!(
                    "Could not load commits: {}",
                    e
                )));
            }
        }

        // Load diff between the refs
        match repo.get_ref_diff_full(&base, &to) {
            Ok(diff_text) => {
                let diffs = parse_diff(&diff_text);
                self.state.modes.pr.diff_view.set_diffs(diffs);
            }
            Err(e) => {
                self.state
                    .notify(Notification::warning(format!("Could not load diff: {}", e)));
            }
        }

        self.state.mark_dirty();
    }

    /// Update Review mode data - load diff between `from_ref` and `to_ref`
    pub fn update_review_data(&mut self, from_ref: Option<String>, to_ref: Option<String>) {
        // Clone the Arc to avoid borrow conflicts with self.state mutations
        let Some(repo) = self.state.repo.clone() else {
            return;
        };

        // Use provided refs or fall back to state
        let from = from_ref.unwrap_or_else(|| self.state.modes.review.from_ref.clone());
        let to = to_ref.unwrap_or_else(|| self.state.modes.review.to_ref.clone());

        // Load diff between the refs
        match repo.get_ref_diff_full(&from, &to) {
            Ok(diff_text) => {
                let diffs = parse_diff(&diff_text);
                self.state.modes.review.diff_view.set_diffs(diffs.clone());

                // Also update file tree from the diff files
                let files: Vec<std::path::PathBuf> = diffs
                    .iter()
                    .map(|d| std::path::PathBuf::from(&d.path))
                    .collect();
                let statuses: Vec<_> = files
                    .iter()
                    .map(|p| (p.clone(), FileGitStatus::Modified))
                    .collect();
                let tree_state = super::components::FileTreeState::from_paths(&files, &statuses);
                self.state.modes.review.file_tree = tree_state;
                self.state.modes.review.file_tree.expand_all();
            }
            Err(e) => {
                self.state
                    .notify(Notification::warning(format!("Could not load diff: {}", e)));
            }
        }

        self.state.mark_dirty();
    }

    /// Update Changelog mode data - load commits and diff between `from_ref` and `to_ref`
    pub fn update_changelog_data(&mut self, from_ref: Option<String>, to_ref: Option<String>) {
        use super::state::ChangelogCommit;

        // Clone the Arc to avoid borrow conflicts with self.state mutations
        let Some(repo) = self.state.repo.clone() else {
            return;
        };

        // Use provided refs or fall back to state
        let from = from_ref.unwrap_or_else(|| self.state.modes.changelog.from_ref.clone());
        let to = to_ref.unwrap_or_else(|| self.state.modes.changelog.to_ref.clone());

        // Load commits between the refs
        match repo.get_commits_between_with_callback(&from, &to, |commit| {
            Ok(ChangelogCommit {
                hash: commit.hash[..7.min(commit.hash.len())].to_string(),
                message: commit.message.lines().next().unwrap_or("").to_string(),
                author: commit.author.clone(),
            })
        }) {
            Ok(commits) => {
                self.state.modes.changelog.commits = commits;
                self.state.modes.changelog.selected_commit = 0;
                self.state.modes.changelog.commit_scroll = 0;
            }
            Err(e) => {
                self.state.notify(Notification::warning(format!(
                    "Could not load commits: {}",
                    e
                )));
            }
        }

        // Load diff between the refs
        match repo.get_ref_diff_full(&from, &to) {
            Ok(diff_text) => {
                let diffs = parse_diff(&diff_text);
                self.state.modes.changelog.diff_view.set_diffs(diffs);
            }
            Err(e) => {
                self.state
                    .notify(Notification::warning(format!("Could not load diff: {}", e)));
            }
        }

        self.state.mark_dirty();
    }

    /// Update release notes mode data when refs change
    pub fn update_release_notes_data(&mut self, from_ref: Option<String>, to_ref: Option<String>) {
        use super::state::ChangelogCommit;

        // Clone the Arc to avoid borrow conflicts with self.state mutations
        let Some(repo) = self.state.repo.clone() else {
            return;
        };

        // Use provided refs or fall back to state
        let from = from_ref.unwrap_or_else(|| self.state.modes.release_notes.from_ref.clone());
        let to = to_ref.unwrap_or_else(|| self.state.modes.release_notes.to_ref.clone());

        // Load commits between the refs
        match repo.get_commits_between_with_callback(&from, &to, |commit| {
            Ok(ChangelogCommit {
                hash: commit.hash[..7.min(commit.hash.len())].to_string(),
                message: commit.message.lines().next().unwrap_or("").to_string(),
                author: commit.author.clone(),
            })
        }) {
            Ok(commits) => {
                self.state.modes.release_notes.commits = commits;
                self.state.modes.release_notes.selected_commit = 0;
                self.state.modes.release_notes.commit_scroll = 0;
            }
            Err(e) => {
                self.state.notify(Notification::warning(format!(
                    "Could not load commits: {}",
                    e
                )));
            }
        }

        // Load diff between the refs
        match repo.get_ref_diff_full(&from, &to) {
            Ok(diff_text) => {
                let diffs = parse_diff(&diff_text);
                self.state.modes.release_notes.diff_view.set_diffs(diffs);
            }
            Err(e) => {
                self.state
                    .notify(Notification::warning(format!("Could not load diff: {}", e)));
            }
        }

        self.state.mark_dirty();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Staging Operations
    // ═══════════════════════════════════════════════════════════════════════════

    /// Stage a single file
    fn stage_file(&mut self, path: &str) {
        let Some(repo) = &self.state.repo else {
            self.state
                .notify(Notification::error("No repository available"));
            return;
        };

        match repo.stage_file(std::path::Path::new(path)) {
            Ok(()) => {
                // Track in companion
                self.state
                    .companion_touch_file(std::path::PathBuf::from(path));
                self.state
                    .notify(Notification::success(format!("Staged: {}", path)));
                let _ = self.refresh_git_status();
                self.state.update_companion_display();
            }
            Err(e) => {
                self.state
                    .notify(Notification::error(format!("Failed to stage: {}", e)));
            }
        }
        self.state.mark_dirty();
    }

    /// Unstage a single file
    fn unstage_file(&mut self, path: &str) {
        let Some(repo) = &self.state.repo else {
            self.state
                .notify(Notification::error("No repository available"));
            return;
        };

        match repo.unstage_file(std::path::Path::new(path)) {
            Ok(()) => {
                // Track in companion
                self.state
                    .companion_touch_file(std::path::PathBuf::from(path));
                self.state
                    .notify(Notification::success(format!("Unstaged: {}", path)));
                let _ = self.refresh_git_status();
                self.state.update_companion_display();
            }
            Err(e) => {
                self.state
                    .notify(Notification::error(format!("Failed to unstage: {}", e)));
            }
        }
        self.state.mark_dirty();
    }

    /// Stage all files
    fn stage_all(&mut self) {
        let Some(repo) = &self.state.repo else {
            self.state
                .notify(Notification::error("No repository available"));
            return;
        };

        // Track all modified files before staging
        let files_to_track: Vec<_> = self
            .state
            .git_status
            .modified_files
            .iter()
            .cloned()
            .chain(self.state.git_status.untracked_files.iter().cloned())
            .collect();

        match repo.stage_all() {
            Ok(()) => {
                // Track all in companion
                for path in files_to_track {
                    self.state.companion_touch_file(path);
                }
                self.state.notify(Notification::success("Staged all files"));
                let _ = self.refresh_git_status();
                self.state.update_companion_display();
            }
            Err(e) => {
                self.state
                    .notify(Notification::error(format!("Failed to stage all: {}", e)));
            }
        }
        self.state.mark_dirty();
    }

    /// Unstage all files
    fn unstage_all(&mut self) {
        let Some(repo) = &self.state.repo else {
            self.state
                .notify(Notification::error("No repository available"));
            return;
        };

        // Track all staged files before unstaging
        let files_to_track: Vec<_> = self.state.git_status.staged_files.clone();

        match repo.unstage_all() {
            Ok(()) => {
                // Track all in companion
                for path in files_to_track {
                    self.state.companion_touch_file(path);
                }
                self.state
                    .notify(Notification::success("Unstaged all files"));
                let _ = self.refresh_git_status();
                self.state.update_companion_display();
            }
            Err(e) => {
                self.state
                    .notify(Notification::error(format!("Failed to unstage all: {}", e)));
            }
        }
        self.state.mark_dirty();
    }

    /// Save settings from the settings modal to config file
    fn save_settings(&mut self) {
        use crate::studio::state::Modal;

        let settings = if let Some(Modal::Settings(s)) = &self.state.modal {
            s.clone()
        } else {
            return;
        };

        if !settings.modified {
            self.state.notify(Notification::info("No changes to save"));
            return;
        }

        // Update config
        let mut config = self.state.config.clone();
        config.default_provider.clone_from(&settings.provider);
        config.use_gitmoji = settings.use_gitmoji;
        config
            .instruction_preset
            .clone_from(&settings.instruction_preset);
        config
            .instructions
            .clone_from(&settings.custom_instructions);
        config.theme.clone_from(&settings.theme);

        // Update provider config
        if let Some(provider_config) = config.providers.get_mut(&settings.provider) {
            provider_config.model.clone_from(&settings.model);
            if let Some(api_key) = &settings.api_key_actual {
                provider_config.api_key.clone_from(api_key);
            }
        }

        // Save to file
        match config.save() {
            Ok(()) => {
                self.state.config = config;
                // Clear the modified flag
                if let Some(Modal::Settings(s)) = &mut self.state.modal {
                    s.modified = false;
                    s.error = None;
                }
                self.state.notify(Notification::success("Settings saved"));
            }
            Err(e) => {
                if let Some(Modal::Settings(s)) = &mut self.state.modal {
                    s.error = Some(format!("Save failed: {}", e));
                }
                self.state
                    .notify(Notification::error(format!("Failed to save: {}", e)));
            }
        }
        self.state.mark_dirty();
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        // Show notification if any
        if let Some(notification) = self.state.current_notification() {
            let style = match notification.level {
                super::state::NotificationLevel::Info => theme::dimmed(),
                super::state::NotificationLevel::Success => theme::success(),
                super::state::NotificationLevel::Warning => theme::warning(),
                super::state::NotificationLevel::Error => theme::error(),
            };
            spans.push(Span::styled(&notification.message, style));
        } else {
            // Context-aware keybinding hints based on mode and panel
            let hints = self.get_context_hints();
            spans.push(Span::styled(hints, theme::dimmed()));
        }

        // Right-align Iris status
        let iris_status = match &self.state.iris_status {
            IrisStatus::Idle => Span::styled("Iris: ready", theme::dimmed()),
            IrisStatus::Thinking { task, .. } => {
                let spinner = self.state.iris_status.spinner_char().unwrap_or('◎');
                Span::styled(
                    format!("{} {}", spinner, task),
                    Style::default().fg(theme::accent_secondary()),
                )
            }
            IrisStatus::Complete { message, .. } => {
                Span::styled(message.clone(), Style::default().fg(theme::success_color()))
            }
            IrisStatus::Error(msg) => Span::styled(format!("Error: {}", msg), theme::error()),
        };

        // Calculate spacing (use saturating_sub to avoid overflow on narrow terminals)
        let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let right_len = iris_status.content.len();
        let padding = (area.width as usize)
            .saturating_sub(left_len)
            .saturating_sub(right_len)
            .saturating_sub(2);
        let padding_str = " ".repeat(padding.max(1));

        spans.push(Span::raw(padding_str));
        spans.push(iris_status);

        let status = Paragraph::new(Line::from(spans));
        frame.render_widget(status, area);
    }

    /// Get context-aware keybinding hints based on mode and focused panel
    fn get_context_hints(&self) -> String {
        let base = "[?]help [Tab]panel [q]quit";

        match self.state.active_mode {
            Mode::Commit => match self.state.focused_panel {
                PanelId::Left => format!("{} · [↑↓]nav [s]stage [u]unstage [a]all [U]reset", base),
                PanelId::Center => format!(
                    "{} · [e]edit [r]regen [p]preset [g]emoji [←→]msg [Enter]commit",
                    base
                ),
                PanelId::Right => format!("{} · [↑↓]scroll [n/p]file []/[]hunk", base),
            },
            Mode::Review | Mode::PR | Mode::Changelog | Mode::ReleaseNotes => {
                match self.state.focused_panel {
                    PanelId::Left => format!("{} · [f/t]set refs [r]generate", base),
                    PanelId::Center => format!("{} · [↑↓]scroll [y]copy [r]generate", base),
                    PanelId::Right => format!("{} · [↑↓]scroll", base),
                }
            }
            Mode::Explore => match self.state.focused_panel {
                PanelId::Left => format!("{} · [↑↓]nav [Enter]open", base),
                PanelId::Center => {
                    format!("{} · [↑↓]nav [v]select [y]copy [Y]copy file [w]why", base)
                }
                PanelId::Right => format!("{} · [c]chat", base),
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Exit Result
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of running the Studio application
#[derive(Debug)]
pub enum ExitResult {
    /// User quit normally
    Quit,
    /// User committed changes (with output message)
    Committed(String),
    /// User amended the previous commit (with output message)
    Amended(String),
    /// An error occurred
    Error(String),
}

impl Drop for StudioApp {
    fn drop(&mut self) {
        // Abort all background tasks to prevent hanging on exit
        for handle in self.background_tasks.drain(..) {
            handle.abort();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Public Entry Point
// ═══════════════════════════════════════════════════════════════════════════════

/// Run Iris Studio
pub fn run_studio(
    config: Config,
    repo: Option<Arc<GitRepo>>,
    commit_service: Option<Arc<GitCommitService>>,
    agent_service: Option<Arc<IrisAgentService>>,
    initial_mode: Option<Mode>,
    from_ref: Option<String>,
    to_ref: Option<String>,
) -> Result<()> {
    // Enable file logging for debugging (TUI owns stdout, so logs go to file only)
    // Only set up default log file if one wasn't specified via CLI (-l --log-file)
    if !crate::logger::has_log_file()
        && let Err(e) = crate::logger::set_log_file(crate::cli::LOG_FILE)
    {
        eprintln!("Warning: Could not set up log file: {}", e);
    }
    // Disable stdout logging - TUI owns the terminal, but debug goes to file
    crate::logger::set_log_to_stdout(false);
    tracing::info!("Iris Studio starting");

    let mut app = StudioApp::new(config, repo, commit_service, agent_service);

    // Set initial mode if specified
    if let Some(mode) = initial_mode {
        app.set_initial_mode(mode);
    }

    // Set comparison refs if specified (applies to Review, PR, Changelog, and Release Notes modes)
    if let Some(from) = from_ref {
        app.state.modes.review.from_ref = from.clone();
        app.state.modes.pr.base_branch = from.clone();
        app.state.modes.changelog.from_ref = from.clone();
        app.state.modes.release_notes.from_ref = from;
    }
    if let Some(to) = to_ref {
        app.state.modes.review.to_ref = to.clone();
        app.state.modes.pr.to_ref = to.clone();
        app.state.modes.changelog.to_ref = to.clone();
        app.state.modes.release_notes.to_ref = to;
    }

    // Run the app
    match app.run()? {
        ExitResult::Quit => {
            // Silent exit
            Ok(())
        }
        ExitResult::Committed(message) => {
            println!("{message}");
            Ok(())
        }
        ExitResult::Amended(message) => {
            println!("{message}");
            Ok(())
        }
        ExitResult::Error(error) => Err(anyhow!("{}", error)),
    }
}
