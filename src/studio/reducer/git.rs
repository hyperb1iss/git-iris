//! Git-related event handlers
//!
//! Handles file staging, unstaging, git status refresh, and file selection.

use std::path::{Path, PathBuf};

use super::super::events::SideEffect;
use super::super::state::{Mode, Notification, StudioState};

/// Handle `FileStaged` event
pub fn file_staged(state: &mut StudioState, path: &Path) -> Vec<SideEffect> {
    state.notify(Notification::success(format!("Staged: {}", path.display())));
    vec![SideEffect::RefreshGitStatus]
}

/// Handle `FileUnstaged` event
pub fn file_unstaged(state: &mut StudioState, path: &Path) -> Vec<SideEffect> {
    state.notify(Notification::info(format!("Unstaged: {}", path.display())));
    vec![SideEffect::RefreshGitStatus]
}

/// Handle `RefreshGitStatus` event
pub fn refresh_git_status() -> Vec<SideEffect> {
    vec![SideEffect::RefreshGitStatus]
}

/// Handle `GitStatusRefreshed` event
pub fn git_status_refreshed(state: &mut StudioState) {
    state.mark_dirty();
}

/// Handle `SelectFile` event
pub fn select_file(state: &mut StudioState, path: PathBuf) {
    match state.active_mode {
        Mode::Explore => {
            state.modes.explore.current_file = Some(path);
        }
        Mode::Commit => {
            // Find index of file in staged files
            if let Some(idx) = state
                .git_status
                .staged_files
                .iter()
                .position(|f| f == &path)
            {
                state.modes.commit.selected_file_index = idx;
            }
        }
        _ => {}
    }
    state.mark_dirty();
}

/// Handle `FileLogLoading` event
pub fn file_log_loading(state: &mut StudioState, path: PathBuf) -> Vec<SideEffect> {
    state.modes.explore.file_log_loading = true;
    state.modes.explore.file_log.clear();
    state.modes.explore.file_log_selected = 0;
    state.modes.explore.file_log_scroll = 0;
    state.mark_dirty();
    vec![SideEffect::LoadFileLog(path)]
}

/// Handle `FileLogLoaded` event
pub fn file_log_loaded(
    state: &mut StudioState,
    file: &Path,
    entries: Vec<crate::studio::state::FileLogEntry>,
) {
    if state.modes.explore.current_file.as_deref() != Some(file) {
        return;
    }
    state.modes.explore.file_log = entries;
    state.modes.explore.file_log_loading = false;
    state.modes.explore.file_log_selected = 0;
    state.modes.explore.file_log_scroll = 0;
    state.mark_dirty();
}

/// Stop loading and report an error only for the current file selection.
pub fn file_log_failed(state: &mut StudioState, file: &Path, error: &str) {
    if state.modes.explore.current_file.as_deref() != Some(file) {
        return;
    }
    state.modes.explore.file_log_loading = false;
    state.notify(Notification::warning(format!(
        "Could not load file history: {error}"
    )));
    state.mark_dirty();
}

/// Handle `GlobalLogLoading` event
pub fn global_log_loading(state: &mut StudioState) -> Vec<SideEffect> {
    state.modes.explore.global_log_loading = true;
    state.modes.explore.global_log.clear();
    state.modes.explore.file_log_selected = 0;
    state.modes.explore.file_log_scroll = 0;
    state.mark_dirty();
    vec![SideEffect::LoadGlobalLog]
}

/// Handle `GlobalLogLoaded` event
pub fn global_log_loaded(
    state: &mut StudioState,
    entries: Vec<crate::studio::state::FileLogEntry>,
) {
    state.modes.explore.global_log = entries;
    state.modes.explore.global_log_loading = false;
    state.modes.explore.file_log_selected = 0;
    state.modes.explore.file_log_scroll = 0;
    state.mark_dirty();
}

/// Handle `ToggleGlobalLog` event
pub fn toggle_global_log(state: &mut StudioState) -> Vec<SideEffect> {
    state.modes.explore.show_global_log = !state.modes.explore.show_global_log;
    state.modes.explore.file_log_selected = 0;
    state.modes.explore.file_log_scroll = 0;

    // Load global log if switching to global view and it's empty
    if state.modes.explore.show_global_log && state.modes.explore.global_log.is_empty() {
        state.modes.explore.global_log_loading = true;
        state.notify(Notification::info("Loading commit log..."));
        state.mark_dirty();
        return vec![SideEffect::LoadGlobalLog];
    }

    let msg = if state.modes.explore.show_global_log {
        "Showing global commit log"
    } else {
        "Showing file history"
    };
    state.notify(Notification::info(msg));
    state.mark_dirty();
    vec![]
}
