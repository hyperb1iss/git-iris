//! Event-driven architecture for Iris Studio
//!
//! All state changes flow through events. This provides:
//! - Clear, traceable data flow
//! - Testable reducer paths for shared workflows
//! - Unified history across all modes
//! - Agent can control UI through tool-emitted events

use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyEvent, MouseEvent};

use crate::types::{GeneratedMessage, Review};

use super::state::{Mode, PanelId};

// Note: Action and IrisQueryRequest are imported directly by reducer.rs from handlers

// ═══════════════════════════════════════════════════════════════════════════════
// Core Event Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Central event type - ALL state changes go through here
#[derive(Debug, Clone)]
pub enum StudioEvent {
    // ─────────────────────────────────────────────────────────────────────────
    // User Input Events
    // ─────────────────────────────────────────────────────────────────────────
    /// Key pressed (already filtered for Press kind)
    KeyPressed(KeyEvent),

    /// Mouse event (click, scroll, etc.)
    Mouse(MouseEvent),

    // ─────────────────────────────────────────────────────────────────────────
    // Navigation Events
    // ─────────────────────────────────────────────────────────────────────────
    /// Switch to a different mode
    SwitchMode(Mode),

    /// Focus a specific panel
    FocusPanel(PanelId),

    /// Cycle focus to next panel
    FocusNext,

    /// Cycle focus to previous panel
    FocusPrev,

    // ─────────────────────────────────────────────────────────────────────────
    // Content Generation Events (user-triggered)
    // ─────────────────────────────────────────────────────────────────────────
    /// Generate commit message
    GenerateCommit {
        instructions: Option<String>,
        preset: String,
        use_gitmoji: bool,
        amend: bool,
    },

    /// Toggle amend mode for commit
    ToggleAmendMode,

    /// Generate code review
    GenerateReview { from_ref: String, to_ref: String },

    /// Generate PR description
    GeneratePR { base_branch: String, to_ref: String },

    /// Generate changelog
    GenerateChangelog { from_ref: String, to_ref: String },

    /// Generate release notes
    GenerateReleaseNotes { from_ref: String, to_ref: String },

    /// Send chat message to Iris
    ChatMessage(String),

    // ─────────────────────────────────────────────────────────────────────────
    // Agent Response Events (from async tasks)
    // ─────────────────────────────────────────────────────────────────────────
    /// Agent task started
    AgentStarted { task_type: TaskType },

    /// Agent is making progress (tool call, etc.)
    AgentProgress {
        task_type: TaskType,
        tool_name: String,
        message: String,
    },

    /// Agent task completed successfully
    AgentComplete {
        task_type: TaskType,
        result: AgentResult,
    },

    /// Agent task failed
    AgentError { task_type: TaskType, error: String },

    /// Streaming text chunk received
    StreamingChunk {
        task_type: TaskType,
        chunk: String,
        aggregated: String,
    },

    /// Streaming completed (final response)
    StreamingComplete { task_type: TaskType },

    /// Dynamic status message received from fast model
    StatusMessage(crate::agents::StatusMessage),

    // ─────────────────────────────────────────────────────────────────────────
    // Tool-Triggered Events (agent controls UI)
    // ─────────────────────────────────────────────────────────────────────────
    /// Update content (from agent tool call)
    UpdateContent {
        content_type: ContentType,
        content: ContentPayload,
    },

    /// Load/refresh data for a mode
    LoadData {
        data_type: DataType,
        from_ref: Option<String>,
        to_ref: Option<String>,
    },

    /// Stage a file (agent can stage files)
    StageFile(PathBuf),

    /// Unstage a file
    UnstageFile(PathBuf),

    // ─────────────────────────────────────────────────────────────────────────
    // File & Git Events
    // ─────────────────────────────────────────────────────────────────────────
    /// File was staged successfully
    FileStaged(PathBuf),

    /// File was unstaged successfully
    FileUnstaged(PathBuf),

    /// Refresh git status
    RefreshGitStatus,

    /// Git status refreshed
    GitStatusRefreshed,

    /// Select a file in the tree
    SelectFile(PathBuf),

    /// File log loaded for explore mode
    FileLogLoaded {
        file: PathBuf,
        entries: Vec<crate::studio::state::FileLogEntry>,
    },

    /// File log loading started
    FileLogLoading(PathBuf),

    /// File history failed to load for a particular selection.
    FileLogFailed { file: PathBuf, error: String },

    /// Global commit log loaded
    GlobalLogLoaded {
        entries: Vec<crate::studio::state::FileLogEntry>,
    },

    /// Global log loading started
    GlobalLogLoading,

    /// Toggle between file log and global log
    ToggleGlobalLog,

    // ─────────────────────────────────────────────────────────────────────────
    // Modal Events
    // ─────────────────────────────────────────────────────────────────────────
    /// Open a modal
    OpenModal(ModalType),

    /// Close current modal
    CloseModal,

    /// Modal action confirmed (e.g., commit confirmed)
    ModalConfirmed {
        modal_type: ModalType,
        data: Option<String>,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // UI Events
    // ─────────────────────────────────────────────────────────────────────────
    /// Show a notification
    Notify {
        level: NotificationLevel,
        message: String,
    },

    /// Scroll content
    Scroll {
        direction: ScrollDirection,
        amount: usize,
    },

    /// Toggle editor mode (view <-> edit)
    ToggleEditMode,

    /// Cycle to next generated message variant
    NextMessageVariant,

    /// Cycle to previous generated message variant
    PrevMessageVariant,

    /// Copy content to clipboard
    CopyToClipboard(String),

    // ─────────────────────────────────────────────────────────────────────────
    // Settings Events
    // ─────────────────────────────────────────────────────────────────────────
    /// Change preset
    SetPreset(String),

    /// Toggle gitmoji
    ToggleGitmoji,

    /// Set custom emoji
    SetEmoji(String),

    // ─────────────────────────────────────────────────────────────────────────
    // Lifecycle Events
    // ─────────────────────────────────────────────────────────────────────────
    /// Request to quit the application
    Quit,

    /// Tick (for animations, polling)
    Tick,

    // ─────────────────────────────────────────────────────────────────────────
    // Companion Events (ambient awareness)
    // ─────────────────────────────────────────────────────────────────────────
    /// File was created in the repository
    CompanionFileCreated(PathBuf),

    /// File was modified in the repository
    CompanionFileModified(PathBuf),

    /// File was deleted from the repository
    CompanionFileDeleted(PathBuf),

    /// Git ref changed (branch switch, commit, etc.)
    CompanionGitRefChanged,

    /// File watcher error occurred
    CompanionWatcherError(String),

    /// Branch switched - show welcome message if returning
    CompanionBranchSwitch {
        branch: String,
        welcome_message: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════════════════════════
// Supporting Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Types of agent tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    Commit,
    Review,
    PR,
    Changelog,
    ReleaseNotes,
    Chat,
    SemanticBlame,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Commit => write!(f, "commit"),
            Self::Review => write!(f, "review"),
            Self::PR => write!(f, "pr"),
            Self::Changelog => write!(f, "changelog"),
            Self::ReleaseNotes => write!(f, "release_notes"),
            Self::Chat => write!(f, "chat"),
            Self::SemanticBlame => write!(f, "semantic_blame"),
        }
    }
}

/// Result from agent task completion
#[derive(Debug, Clone)]
pub enum AgentResult {
    /// Commit message(s) generated
    CommitMessages(Vec<GeneratedMessage>),

    /// Review generated
    ReviewContent(Review),

    /// PR description generated
    PRContent(String),

    /// Changelog generated
    ChangelogContent(String),

    /// Release notes generated
    ReleaseNotesContent(String),

    /// Chat response
    ChatResponse(String),

    /// Semantic blame explanation
    SemanticBlame(SemanticBlameResult),
}

/// Result from semantic blame query
#[derive(Debug, Clone)]
pub struct SemanticBlameResult {
    /// The file that was analyzed
    pub file: PathBuf,
    /// Line range analyzed
    pub start_line: usize,
    pub end_line: usize,
    /// The commit that introduced this code
    pub commit_hash: String,
    /// Commit author
    pub author: String,
    /// When the commit was made
    pub commit_date: String,
    /// Original commit message
    pub commit_message: String,
    /// AI-generated explanation of why this code exists
    pub explanation: String,
}

/// Types of content that can be updated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    CommitMessage,
    PRDescription,
    CodeReview,
    Changelog,
    ReleaseNotes,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommitMessage => write!(f, "commit_message"),
            Self::PRDescription => write!(f, "pr_description"),
            Self::CodeReview => write!(f, "code_review"),
            Self::Changelog => write!(f, "changelog"),
            Self::ReleaseNotes => write!(f, "release_notes"),
        }
    }
}

/// Content payload for updates
#[derive(Debug, Clone)]
pub enum ContentPayload {
    /// Structured commit message
    Commit(GeneratedMessage),

    /// Markdown content (PR, review, changelog, release notes)
    Markdown(String),
}

/// Types of data that can be loaded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    GitStatus,
    CommitDiff,
    ReviewDiff,
    PRDiff,
    ChangelogCommits,
    ReleaseNotesCommits,
    /// Explore mode file tree (lazy-loaded on mode switch)
    ExploreFiles,
}

/// Modal types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalType {
    Help,
    Chat,
    Settings,
    PresetSelector,
    EmojiSelector,
    RefSelector { field: RefField },
    ConfirmCommit,
    ConfirmAmend,
    ConfirmQuit,
}

/// Which ref field is being edited
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefField {
    From,
    To,
    Base,
}

/// Notification severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Side Effects
// ═══════════════════════════════════════════════════════════════════════════════

/// Side effects produced by the reducer
///
/// These are executed by the app after state is updated.
/// This keeps the reducer pure (no I/O).
#[derive(Debug, Clone)]
pub enum SideEffect {
    /// Spawn an agent task
    SpawnAgent { task: AgentTask },

    /// Load data asynchronously
    LoadData {
        data_type: DataType,
        from_ref: Option<String>,
        to_ref: Option<String>,
    },

    /// Stage a file in git
    GitStage(PathBuf),

    /// Unstage a file in git
    GitUnstage(PathBuf),

    /// Stage all files
    GitStageAll,

    /// Unstage all files
    GitUnstageAll,

    /// Save settings to config
    SaveSettings,

    /// Refresh git status
    RefreshGitStatus,

    /// Copy to system clipboard
    CopyToClipboard(String),

    /// Execute git commit
    ExecuteCommit { message: String },

    /// Execute git commit --amend
    ExecuteAmend { message: String },

    /// Show notification (if needs timing/animation)
    #[allow(dead_code)] // Kept for future use - handled in executor but not yet constructed
    ShowNotification {
        level: NotificationLevel,
        message: String,
        duration_ms: u64,
    },

    /// Request terminal redraw
    #[allow(dead_code)] // Kept for future use - handled in executor but not yet constructed
    Redraw,

    /// Quit the application
    Quit,

    /// Gather blame info for a file region and spawn semantic blame agent
    GatherBlameAndSpawnAgent {
        file: PathBuf,
        start_line: usize,
        end_line: usize,
    },

    /// Load file git log for the right panel in explore mode
    LoadFileLog(PathBuf),

    /// Load global commit log (not file-specific)
    LoadGlobalLog,
}

/// Blame information gathered from git
#[derive(Debug, Clone)]
pub struct BlameInfo {
    /// The file that was blamed
    pub file: PathBuf,
    /// Start line of the region
    pub start_line: usize,
    /// End line of the region
    pub end_line: usize,
    /// Commit hash that introduced the code
    pub commit_hash: String,
    /// Author name
    pub author: String,
    /// Commit date
    pub commit_date: String,
    /// Original commit message
    pub commit_message: String,
    /// The actual code content in the range
    pub code_content: String,
}

/// Agent task to spawn
#[derive(Debug, Clone)]
pub enum AgentTask {
    Commit {
        instructions: Option<String>,
        preset: String,
        use_gitmoji: bool,
        amend: bool,
    },
    Review {
        from_ref: String,
        to_ref: String,
    },
    PR {
        base_branch: String,
        to_ref: String,
    },
    Changelog {
        from_ref: String,
        to_ref: String,
    },
    ReleaseNotes {
        from_ref: String,
        to_ref: String,
    },
    Chat {
        message: String,
        context: ChatContext,
    },
    #[allow(dead_code)]
    // Kept for future use - currently GatherBlameAndSpawnAgent handles this directly
    SemanticBlame {
        blame_info: BlameInfo,
    },
}

/// Context for chat messages
#[derive(Debug, Clone, Default)]
pub struct ChatContext {
    /// Current mode when chat was opened
    pub mode: Mode,
    /// Current content being discussed
    pub current_content: Option<String>,
    /// Diff summary for context
    #[allow(dead_code)] // Kept for future use - will provide diff context to chat
    pub diff_summary: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Event Source Tracking
// ═══════════════════════════════════════════════════════════════════════════════

/// Where an event originated from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    /// User input (keyboard, mouse)
    User,
    /// Agent/LLM response
    Agent,
    /// Tool call from agent
    Tool,
    /// System (tick, refresh, etc.)
    System,
}

/// Timestamped event for history
#[derive(Debug, Clone)]
pub struct TimestampedEvent {
    pub timestamp: Instant,
    pub source: EventSource,
    pub event: StudioEvent,
}

impl TimestampedEvent {
    pub fn user(event: StudioEvent) -> Self {
        Self {
            timestamp: Instant::now(),
            source: EventSource::User,
            event,
        }
    }

    pub fn agent(event: StudioEvent) -> Self {
        Self {
            timestamp: Instant::now(),
            source: EventSource::Agent,
            event,
        }
    }

    pub fn tool(event: StudioEvent) -> Self {
        Self {
            timestamp: Instant::now(),
            source: EventSource::Tool,
            event,
        }
    }

    pub fn system(event: StudioEvent) -> Self {
        Self {
            timestamp: Instant::now(),
            source: EventSource::System,
            event,
        }
    }
}
