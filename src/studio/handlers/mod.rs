//! Event handlers for Iris Studio
//!
//! Handlers process keyboard input and return side effects.
//! State mutations happen directly; effects are returned for async/IO operations.

mod changelog;
mod commit;
mod explore;
mod modals;
mod pr;
mod release_notes;
mod review;

use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::studio::events::{AgentTask, ChatContext, DataType, SideEffect};
use crate::studio::state::{Modal, Mode, Notification, SettingsState, StudioState};

pub use changelog::handle_changelog_key;
pub use commit::handle_commit_key;
pub use explore::handle_explore_key;
pub(crate) use explore::load_selected_file;
pub use modals::handle_modal_key;
pub use pr::handle_pr_key;
pub use release_notes::handle_release_notes_key;
pub use review::handle_review_key;

// ═══════════════════════════════════════════════════════════════════════════════
// Main Event Handler
// ═══════════════════════════════════════════════════════════════════════════════

/// Process a key event and return any side effects needed
pub fn handle_key_event(state: &mut StudioState, key: KeyEvent) -> Vec<SideEffect> {
    // Handle modals first
    if state.modal.is_some() {
        return handle_modal_key(state, key);
    }

    // Global keybindings (work in all modes)
    if let Some(effects) = handle_global_key(state, key) {
        return effects;
    }

    // Mode-specific keybindings
    match state.active_mode {
        Mode::Explore => handle_explore_key(state, key),
        Mode::Commit => handle_commit_key(state, key),
        Mode::Review => handle_review_key(state, key),
        Mode::PR => handle_pr_key(state, key),
        Mode::Changelog => handle_changelog_key(state, key),
        Mode::ReleaseNotes => handle_release_notes_key(state, key),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Global Key Handling
// ═══════════════════════════════════════════════════════════════════════════════

fn handle_global_key(state: &mut StudioState, key: KeyEvent) -> Option<Vec<SideEffect>> {
    if matches_shift_char(&key, 'e') {
        return Some(switch_mode(state, Mode::Explore));
    }
    if matches_shift_char(&key, 'c') {
        return Some(switch_mode(state, Mode::Commit));
    }
    if matches_shift_char(&key, 'r') {
        return Some(switch_mode(state, Mode::Review));
    }
    if matches_shift_char(&key, 'p') {
        return Some(switch_mode(state, Mode::PR));
    }
    if matches_shift_char(&key, 'l') {
        return Some(switch_mode(state, Mode::Changelog));
    }
    if matches_shift_char(&key, 'n') {
        return Some(switch_mode(state, Mode::ReleaseNotes));
    }
    if matches_shift_char(&key, 's') {
        state.modal = Some(Modal::Settings(Box::new(SettingsState::from_config(
            &state.config,
        ))));
        state.mark_dirty();
        return Some(vec![]);
    }

    match key.code {
        // Quit
        KeyCode::Char('q') if !is_editing(state) => Some(vec![SideEffect::Quit]),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(vec![SideEffect::Quit])
        }

        // Help
        KeyCode::Char('?') if !is_editing(state) => {
            state.show_help();
            Some(vec![])
        }

        // Chat with Iris
        KeyCode::Char('/') if !is_editing(state) => {
            state.show_chat();
            Some(vec![])
        }

        // Panel navigation
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                state.focus_prev_panel();
            } else {
                state.focus_next_panel();
            }
            Some(vec![])
        }

        // Escape closes modals or cancels current operation
        KeyCode::Esc => {
            if state.modal.is_some() {
                state.close_modal();
                Some(vec![])
            } else {
                // Mode-specific escape handling
                None
            }
        }

        _ => None,
    }
}

fn matches_shift_char(key: &KeyEvent, expected: char) -> bool {
    *key == KeyEvent::new_with_kind_and_state(
        KeyCode::Char(expected),
        KeyModifiers::SHIFT,
        key.kind,
        key.state,
    )
}

/// Switch mode and return appropriate data loading effect
fn switch_mode(state: &mut StudioState, mode: Mode) -> Vec<SideEffect> {
    if state.active_mode == mode {
        return vec![];
    }

    state.switch_mode(mode);

    match mode {
        Mode::Commit => vec![SideEffect::LoadData {
            data_type: DataType::CommitDiff,
            from_ref: None,
            to_ref: None,
        }],
        Mode::Review => vec![SideEffect::LoadData {
            data_type: DataType::ReviewDiff,
            from_ref: Some(state.modes.review.from_ref.clone()),
            to_ref: Some(state.modes.review.to_ref.clone()),
        }],
        Mode::PR => vec![SideEffect::LoadData {
            data_type: DataType::PRDiff,
            from_ref: Some(state.modes.pr.base_branch.clone()),
            to_ref: Some(state.modes.pr.to_ref.clone()),
        }],
        Mode::Changelog => vec![SideEffect::LoadData {
            data_type: DataType::ChangelogCommits,
            from_ref: Some(state.modes.changelog.from_ref.clone()),
            to_ref: Some(state.modes.changelog.to_ref.clone()),
        }],
        Mode::ReleaseNotes => vec![SideEffect::LoadData {
            data_type: DataType::ReleaseNotesCommits,
            from_ref: Some(state.modes.release_notes.from_ref.clone()),
            to_ref: Some(state.modes.release_notes.to_ref.clone()),
        }],
        Mode::Explore => {
            // Load file tree if not already populated
            if state.modes.explore.file_tree.is_empty() {
                vec![SideEffect::LoadData {
                    data_type: DataType::ExploreFiles,
                    from_ref: None,
                    to_ref: None,
                }]
            } else {
                vec![]
            }
        }
    }
}

/// Check if we're in an editing state (text input mode)
pub fn is_editing(state: &StudioState) -> bool {
    match state.active_mode {
        Mode::Commit => state.modes.commit.editing_message,
        _ => false,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Keybinding Descriptions
// ═══════════════════════════════════════════════════════════════════════════════

/// Get keybinding descriptions for help display
#[allow(dead_code)] // Will be used for dynamic help overlay
pub fn get_keybindings(mode: Mode) -> Vec<(&'static str, &'static str)> {
    let mut bindings = vec![
        // Global
        ("q", "Quit"),
        ("?", "Help"),
        ("Tab", "Next panel"),
        ("S-Tab", "Previous panel"),
        ("/", "Search"),
        ("E", "Explore mode"),
        ("C", "Commit mode"),
    ];

    // Mode-specific
    match mode {
        Mode::Explore => {
            bindings.extend([
                ("j/k", "Navigate up/down"),
                ("h/l", "Collapse/expand"),
                ("g/G", "First/last"),
                ("Enter", "Open/select"),
                ("w", "Ask why"),
                ("H", "Toggle heat map"),
                ("o", "Copy editor command"),
            ]);
        }
        Mode::Commit => {
            bindings.extend([
                ("j/k", "Navigate/scroll"),
                ("h/l", "Collapse/expand"),
                ("[/]", "Prev/next hunk"),
                ("n/p", "Cycle diff files"),
                ("Left/Right", "Cycle messages"),
                ("s", "Stage file"),
                ("u", "Unstage file"),
                ("a", "Stage all"),
                ("U", "Unstage all"),
                ("e", "Edit message"),
                ("r", "Regenerate"),
                ("R", "Reset message"),
                ("Enter", "Commit/select"),
            ]);
        }
        _ => {}
    }

    bindings
}

// ═══════════════════════════════════════════════════════════════════════════════
// Clipboard Utilities
// ═══════════════════════════════════════════════════════════════════════════════

/// Copy text to the system clipboard and notify the user
pub fn copy_to_clipboard(state: &mut StudioState, content: &str, description: &str) {
    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(content) {
            Ok(()) => {
                state.notify(Notification::success(format!(
                    "{description} copied to clipboard"
                )));
            }
            Err(e) => {
                state.notify(Notification::error(format!("Failed to copy: {e}")));
            }
        },
        Err(e) => {
            state.notify(Notification::error(format!("Clipboard unavailable: {e}")));
        }
    }
    state.mark_dirty();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper: Create Agent Tasks
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a commit generation agent task
pub fn spawn_commit_task(state: &StudioState) -> SideEffect {
    use crate::studio::state::EmojiMode;

    SideEffect::SpawnAgent {
        task: AgentTask::Commit {
            instructions: if state.modes.commit.custom_instructions.is_empty() {
                None
            } else {
                Some(state.modes.commit.custom_instructions.clone())
            },
            preset: state.modes.commit.preset.clone(),
            use_gitmoji: state.modes.commit.emoji_mode != EmojiMode::None,
            amend: state.modes.commit.amend_mode,
        },
    }
}

/// Create a review generation agent task
pub fn spawn_review_task(state: &StudioState) -> SideEffect {
    SideEffect::SpawnAgent {
        task: AgentTask::Review {
            from_ref: state.modes.review.from_ref.clone(),
            to_ref: state.modes.review.to_ref.clone(),
        },
    }
}

/// Create a PR generation agent task
pub fn spawn_pr_task(state: &StudioState) -> SideEffect {
    SideEffect::SpawnAgent {
        task: AgentTask::PR {
            base_branch: state.modes.pr.base_branch.clone(),
            to_ref: state.modes.pr.to_ref.clone(),
        },
    }
}

/// Create a changelog generation agent task
pub fn spawn_changelog_task(state: &StudioState) -> SideEffect {
    SideEffect::SpawnAgent {
        task: AgentTask::Changelog {
            from_ref: state.modes.changelog.from_ref.clone(),
            to_ref: state.modes.changelog.to_ref.clone(),
        },
    }
}

/// Create a release notes generation agent task
pub fn spawn_release_notes_task(state: &StudioState) -> SideEffect {
    SideEffect::SpawnAgent {
        task: AgentTask::ReleaseNotes {
            from_ref: state.modes.release_notes.from_ref.clone(),
            to_ref: state.modes.release_notes.to_ref.clone(),
        },
    }
}

/// Create a chat agent task
pub fn spawn_chat_task(message: String, mode: Mode) -> SideEffect {
    SideEffect::SpawnAgent {
        task: AgentTask::Chat {
            message,
            context: ChatContext {
                mode,
                ..Default::default()
            },
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper: Data Reload Effects
// ═══════════════════════════════════════════════════════════════════════════════

/// Get reload effect for PR data
pub fn reload_pr_data(state: &StudioState) -> SideEffect {
    SideEffect::LoadData {
        data_type: DataType::PRDiff,
        from_ref: Some(state.modes.pr.base_branch.clone()),
        to_ref: Some(state.modes.pr.to_ref.clone()),
    }
}

/// Get reload effect for Review data
pub fn reload_review_data(state: &StudioState) -> SideEffect {
    SideEffect::LoadData {
        data_type: DataType::ReviewDiff,
        from_ref: Some(state.modes.review.from_ref.clone()),
        to_ref: Some(state.modes.review.to_ref.clone()),
    }
}

/// Get reload effect for Changelog data
pub fn reload_changelog_data(state: &StudioState) -> SideEffect {
    SideEffect::LoadData {
        data_type: DataType::ChangelogCommits,
        from_ref: Some(state.modes.changelog.from_ref.clone()),
        to_ref: Some(state.modes.changelog.to_ref.clone()),
    }
}

/// Get reload effect for Release Notes data
pub fn reload_release_notes_data(state: &StudioState) -> SideEffect {
    SideEffect::LoadData {
        data_type: DataType::ReleaseNotesCommits,
        from_ref: Some(state.modes.release_notes.from_ref.clone()),
        to_ref: Some(state.modes.release_notes.to_ref.clone()),
    }
}
