//! Explore mode key handling for Iris Studio

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;

use crate::studio::events::SideEffect;
use crate::studio::state::{Notification, PanelId, StudioState};

/// Default visible height for code view navigation (will be adjusted by actual render)
const DEFAULT_VISIBLE_HEIGHT: usize = 30;

/// Handle key events in Explore mode
pub fn handle_explore_key(state: &mut StudioState, key: KeyEvent) -> Vec<SideEffect> {
    // Global explore mode keys (work in any panel)
    // Toggle between file log and global commit log
    if let KeyCode::Char('L') = key.code {
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
            "Showing global commit log (L to toggle)"
        } else {
            "Showing file history (L to toggle)"
        };
        state.notify(Notification::info(msg));
        state.mark_dirty();
        return vec![];
    }

    // Panel-specific keys
    match state.focused_panel {
        PanelId::Left => handle_file_tree_key(state, key),
        PanelId::Center => handle_code_view_key(state, key),
        PanelId::Right => handle_context_key(state, key),
    }
}

/// Load the selected file into the code view and trigger file log loading
pub(crate) fn load_selected_file(state: &mut StudioState) -> Vec<SideEffect> {
    if let Some(entry) = state.modes.explore.file_tree.selected_entry()
        && !entry.is_dir
    {
        let path = entry.path.clone();
        state.modes.explore.current_file = Some(path.clone());
        if let Err(e) = state.modes.explore.code_view.load_file(&path) {
            state.notify(Notification::warning(format!("Could not load file: {}", e)));
        }
        // Trigger file log loading
        state.modes.explore.file_log_loading = true;
        state.modes.explore.file_log.clear();
        state.modes.explore.file_log_selected = 0;
        state.modes.explore.file_log_scroll = 0;
        return vec![SideEffect::LoadFileLog(path)];
    }
    vec![]
}

fn handle_file_tree_key(state: &mut StudioState, key: KeyEvent) -> Vec<SideEffect> {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            state.modes.explore.file_tree.select_next();
            let effects = load_selected_file(state);
            state.mark_dirty();
            effects
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.modes.explore.file_tree.select_prev();
            let effects = load_selected_file(state);
            state.mark_dirty();
            effects
        }
        KeyCode::Char('h') | KeyCode::Left => {
            state.modes.explore.file_tree.collapse();
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('l') | KeyCode::Right => {
            state.modes.explore.file_tree.expand();
            state.mark_dirty();
            vec![]
        }
        KeyCode::Enter => {
            // Toggle expand for directories, or select file and move to code view
            let effects = if let Some(entry) = state.modes.explore.file_tree.selected_entry() {
                if entry.is_dir {
                    state.modes.explore.file_tree.toggle_expand();
                    vec![]
                } else {
                    let effects = load_selected_file(state);
                    state.focus_next_panel(); // Move to code view
                    effects
                }
            } else {
                vec![]
            };
            state.mark_dirty();
            effects
        }
        KeyCode::Char('g') | KeyCode::Home => {
            // Go to first
            state.modes.explore.file_tree.select_first();
            let effects = load_selected_file(state);
            state.mark_dirty();
            effects
        }
        KeyCode::Char('G') | KeyCode::End => {
            // Go to last
            state.modes.explore.file_tree.select_last();
            let effects = load_selected_file(state);
            state.mark_dirty();
            effects
        }
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.modes.explore.file_tree.page_down(10);
            let effects = load_selected_file(state);
            state.mark_dirty();
            effects
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.modes.explore.file_tree.page_up(10);
            let effects = load_selected_file(state);
            state.mark_dirty();
            effects
        }

        _ => vec![],
    }
}

/// Update selection range based on anchor and current line
fn update_visual_selection(state: &mut StudioState) {
    if let Some(anchor) = state.modes.explore.selection_anchor {
        let current = state.modes.explore.current_line;
        let (start, end) = if current < anchor {
            (current, anchor)
        } else {
            (anchor, current)
        };
        state.modes.explore.selection = Some((start, end));
        state.modes.explore.code_view.set_selection(start, end);
    }
}

/// Clear visual selection state
fn clear_selection(state: &mut StudioState) {
    state.modes.explore.selection = None;
    state.modes.explore.selection_anchor = None;
    state.modes.explore.code_view.clear_selection();
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn editor_hint_command(editor: &str, line: usize, path: &Path) -> String {
    format!("{editor} +{line} {}", shell_quote(&path.to_string_lossy()))
}

fn git_show_command(hash: &str, file: Option<&Path>) -> String {
    match file {
        Some(path) => format!(
            "git show {hash} -- {}",
            shell_quote(&path.to_string_lossy())
        ),
        None => format!("git show {hash}"),
    }
}

fn handle_code_view_key(state: &mut StudioState, key: KeyEvent) -> Vec<SideEffect> {
    match key.code {
        // Navigation - single line
        KeyCode::Char('j') | KeyCode::Down => {
            state
                .modes
                .explore
                .code_view
                .move_down(1, DEFAULT_VISIBLE_HEIGHT);
            state.modes.explore.current_line = state.modes.explore.code_view.selected_line();
            // Extend selection if in visual mode
            if state.modes.explore.selection_anchor.is_some() {
                update_visual_selection(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state
                .modes
                .explore
                .code_view
                .move_up(1, DEFAULT_VISIBLE_HEIGHT);
            state.modes.explore.current_line = state.modes.explore.code_view.selected_line();
            // Extend selection if in visual mode
            if state.modes.explore.selection_anchor.is_some() {
                update_visual_selection(state);
            }
            state.mark_dirty();
            vec![]
        }
        // Page navigation
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state
                .modes
                .explore
                .code_view
                .move_down(20, DEFAULT_VISIBLE_HEIGHT);
            state.modes.explore.current_line = state.modes.explore.code_view.selected_line();
            if state.modes.explore.selection_anchor.is_some() {
                update_visual_selection(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state
                .modes
                .explore
                .code_view
                .move_up(20, DEFAULT_VISIBLE_HEIGHT);
            state.modes.explore.current_line = state.modes.explore.code_view.selected_line();
            if state.modes.explore.selection_anchor.is_some() {
                update_visual_selection(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('g') | KeyCode::Home => {
            // Go to first line
            state.modes.explore.code_view.goto_first();
            state.modes.explore.current_line = 1;
            if state.modes.explore.selection_anchor.is_some() {
                update_visual_selection(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('G') | KeyCode::End => {
            // Go to last line
            state
                .modes
                .explore
                .code_view
                .goto_last(DEFAULT_VISIBLE_HEIGHT);
            state.modes.explore.current_line = state.modes.explore.code_view.selected_line();
            if state.modes.explore.selection_anchor.is_some() {
                update_visual_selection(state);
            }
            state.mark_dirty();
            vec![]
        }

        // Visual selection mode (vim-style 'v')
        KeyCode::Char('v') => {
            if state.modes.explore.selection_anchor.is_some() {
                // Already in visual mode, exit it
                clear_selection(state);
                state.notify(Notification::info("Selection cleared"));
            } else {
                // Enter visual mode
                let current = state.modes.explore.current_line;
                state.modes.explore.selection_anchor = Some(current);
                state.modes.explore.selection = Some((current, current));
                state
                    .modes
                    .explore
                    .code_view
                    .set_selection(current, current);
                state.notify(Notification::info(
                    "Visual mode: use j/k to select, y to copy, Esc to cancel",
                ));
            }
            state.mark_dirty();
            vec![]
        }

        // Escape - clear selection
        KeyCode::Esc => {
            if state.modes.explore.selection_anchor.is_some() {
                clear_selection(state);
                state.notify(Notification::info("Selection cleared"));
                state.mark_dirty();
            }
            vec![]
        }

        // Heat map toggle
        KeyCode::Char('H') => {
            state.modes.explore.show_heat_map = !state.modes.explore.show_heat_map;
            state.mark_dirty();
            vec![]
        }

        // Ask "why" about current line - semantic blame
        KeyCode::Char('w') => {
            let file = state.modes.explore.current_file.clone();
            if let Some(file) = file {
                if state.modes.explore.blame_loading {
                    state.notify(Notification::info("Already analyzing..."));
                    state.mark_dirty();
                    vec![]
                } else {
                    let line = state.modes.explore.current_line;
                    let end_line = state.modes.explore.selection.map_or(line, |(_, end)| end);

                    // Set loading state
                    state.modes.explore.blame_loading = true;
                    state.set_iris_thinking("Analyzing code history...");
                    state.mark_dirty();

                    vec![SideEffect::GatherBlameAndSpawnAgent {
                        file,
                        start_line: line,
                        end_line,
                    }]
                }
            } else {
                state.notify(Notification::warning("No file selected"));
                state.mark_dirty();
                vec![]
            }
        }

        // Copy selection or current line to clipboard
        KeyCode::Char('y') => {
            let lines = state.modes.explore.code_view.lines();
            let content = if let Some((start, end)) = state.modes.explore.selection {
                // Copy selected lines
                let start_idx = start.saturating_sub(1);
                let end_idx = end.min(lines.len());
                let selected: Vec<&str> = lines
                    .iter()
                    .skip(start_idx)
                    .take(end_idx - start_idx)
                    .map(String::as_str)
                    .collect();
                if selected.is_empty() {
                    None
                } else {
                    Some((selected.join("\n"), selected.len()))
                }
            } else {
                // Copy current line
                let line_idx = state
                    .modes
                    .explore
                    .code_view
                    .selected_line()
                    .saturating_sub(1);
                lines.get(line_idx).map(|s| (s.clone(), 1))
            };

            if let Some((text, line_count)) = content {
                let msg = if line_count > 1 {
                    format!("{} lines copied to clipboard", line_count)
                } else {
                    "Line copied to clipboard".to_string()
                };
                state.notify(Notification::success(msg));
                // Clear selection after copying (vim behavior)
                clear_selection(state);
                state.mark_dirty();
                vec![SideEffect::CopyToClipboard(text)]
            } else {
                state.notify(Notification::warning("Nothing to copy"));
                state.mark_dirty();
                vec![]
            }
        }

        // Copy entire file content
        KeyCode::Char('Y') => {
            let content = state.modes.explore.code_view.lines().join("\n");
            if content.is_empty() {
                state.notify(Notification::warning("No content to copy"));
                state.mark_dirty();
                vec![]
            } else {
                state.notify(Notification::success("File content copied to clipboard"));
                state.mark_dirty();
                vec![SideEffect::CopyToClipboard(content)]
            }
        }

        // Copy an $EDITOR command for the current file/line
        KeyCode::Char('o') => {
            if let Some(path) = state.modes.explore.current_file.clone() {
                let editor = std::env::var("VISUAL")
                    .ok()
                    .filter(|value| !value.is_empty())
                    .or_else(|| {
                        std::env::var("EDITOR")
                            .ok()
                            .filter(|value| !value.is_empty())
                    })
                    .unwrap_or_else(|| "vim".to_string());
                let command = editor_hint_command(&editor, state.modes.explore.current_line, &path);
                state.notify(Notification::success("Editor command copied"));
                state.mark_dirty();
                vec![SideEffect::CopyToClipboard(command)]
            } else {
                state.notify(Notification::warning("No file selected"));
                state.mark_dirty();
                vec![]
            }
        }

        _ => vec![],
    }
}

/// Visible entries in file log panel (2 lines per entry, assume ~15 entries visible)
const FILE_LOG_VISIBLE_ENTRIES: usize = 15;

/// Adjust scroll to keep selection visible
fn adjust_file_log_scroll(state: &mut StudioState) {
    let selected = state.modes.explore.file_log_selected;
    let scroll = &mut state.modes.explore.file_log_scroll;
    let visible = FILE_LOG_VISIBLE_ENTRIES;

    // Scroll down if selection is below visible area
    if selected >= *scroll + visible {
        *scroll = selected.saturating_sub(visible - 1);
    }
    // Scroll up if selection is above visible area
    if selected < *scroll {
        *scroll = selected;
    }
}

fn handle_context_key(state: &mut StudioState, key: KeyEvent) -> Vec<SideEffect> {
    let show_global_log = state.modes.explore.show_global_log;
    let active_log = if show_global_log {
        &state.modes.explore.global_log
    } else {
        &state.modes.explore.file_log
    };
    let log_len = active_log.len();

    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            if log_len > 0 {
                let selected = &mut state.modes.explore.file_log_selected;
                if *selected < log_len.saturating_sub(1) {
                    *selected += 1;
                }
                adjust_file_log_scroll(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if log_len > 0 {
                let selected = &mut state.modes.explore.file_log_selected;
                if *selected > 0 {
                    *selected -= 1;
                }
                adjust_file_log_scroll(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('g') | KeyCode::Home => {
            state.modes.explore.file_log_selected = 0;
            state.modes.explore.file_log_scroll = 0;
            state.mark_dirty();
            vec![]
        }
        KeyCode::Char('G') | KeyCode::End => {
            if log_len > 0 {
                state.modes.explore.file_log_selected = log_len.saturating_sub(1);
                adjust_file_log_scroll(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if log_len > 0 {
                let selected = &mut state.modes.explore.file_log_selected;
                *selected = (*selected + 10).min(log_len.saturating_sub(1));
                adjust_file_log_scroll(state);
            }
            state.mark_dirty();
            vec![]
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let selected = &mut state.modes.explore.file_log_selected;
            *selected = selected.saturating_sub(10);
            adjust_file_log_scroll(state);
            state.mark_dirty();
            vec![]
        }
        KeyCode::Enter => {
            if log_len > 0 {
                let selected = state.modes.explore.file_log_selected;
                if let Some(entry) = active_log.get(selected) {
                    let file = if show_global_log {
                        None
                    } else {
                        state.modes.explore.current_file.as_deref()
                    };
                    let command = git_show_command(&entry.hash, file);
                    state.notify(Notification::success("git show command copied"));
                    return vec![SideEffect::CopyToClipboard(command)];
                }
            }
            vec![]
        }
        KeyCode::Char('y') => {
            // Copy selected commit hash
            if log_len > 0 {
                let selected = state.modes.explore.file_log_selected;
                if let Some(entry) = active_log.get(selected) {
                    let hash = entry.short_hash.clone();
                    state.notify(Notification::success("Commit hash copied"));
                    return vec![SideEffect::CopyToClipboard(hash)];
                }
            }
            vec![]
        }

        _ => vec![],
    }
}
