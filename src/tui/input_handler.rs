use super::app::TuiCommit;
use super::spinner::SpinnerState;
use super::state::{Mode, UserInfoFocus};
use crate::context::format_commit_message;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

pub fn handle_input(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match app.state.mode {
        Mode::Normal => {
            let result = handle_normal_mode(app, key);
            app.state.dirty = true; // Mark dirty after handling input
            result
        }
        Mode::EditingMessage => {
            let result = handle_editing_message(app, key);
            app.state.dirty = true; // Mark dirty after handling input
            result
        }
        Mode::EditingInstructions => handle_editing_instructions(app, key),
        Mode::SelectingEmoji => handle_selecting_emoji(app, key),
        Mode::SelectingPreset => handle_selecting_preset(app, key),
        Mode::EditingUserInfo => handle_editing_user_info(app, key),
        Mode::Generating => {
            if key.code == KeyCode::Esc {
                app.state.mode = Mode::Normal;
                app.state
                    .set_status(String::from("Message generation cancelled."));
            }
            InputResult::Continue
        }
    }
}

fn handle_normal_mode(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Char('e') => {
            app.state.mode = Mode::EditingMessage;
            app.state
                .set_status(String::from("Editing commit message. Press Esc to finish."));
            InputResult::Continue
        }
        KeyCode::Char('i') => {
            app.state.mode = Mode::EditingInstructions;
            app.state
                .set_status(String::from("Editing instructions. Press Esc to finish."));
            InputResult::Continue
        }
        KeyCode::Char('g') => {
            app.state.mode = Mode::SelectingEmoji;
            app.state.set_status(String::from(
                "Selecting emoji. Use arrow keys and Enter to select, Esc to cancel.",
            ));
            InputResult::Continue
        }
        KeyCode::Char('p') => {
            app.state.mode = Mode::SelectingPreset;
            app.state.set_status(String::from(
                "Selecting preset. Use arrow keys and Enter to select, Esc to cancel.",
            ));
            InputResult::Continue
        }
        KeyCode::Char('u') => {
            app.state.mode = Mode::EditingUserInfo;
            app.state.set_status(String::from(
                "Editing user info. Press Tab to switch fields, Enter to save, Esc to cancel.",
            ));
            InputResult::Continue
        }
        KeyCode::Char('r') => {
            app.handle_regenerate();
            InputResult::Continue
        }
        KeyCode::Left => {
            if app.state.current_index > 0 {
                app.state.current_index -= 1;
            } else {
                app.state.current_index = app.state.messages.len() - 1;
            }
            app.state.update_message_textarea();
            app.state.set_status(format!(
                "Viewing commit message {}/{}",
                app.state.current_index + 1,
                app.state.messages.len()
            ));
            InputResult::Continue
        }
        KeyCode::Right => {
            if app.state.current_index < app.state.messages.len() - 1 {
                app.state.current_index += 1;
            } else {
                app.state.current_index = 0;
            }
            app.state.update_message_textarea();
            app.state.set_status(format!(
                "Viewing commit message {}/{}",
                app.state.current_index + 1,
                app.state.messages.len()
            ));
            InputResult::Continue
        }
        KeyCode::Enter => {
            let commit_message =
                format_commit_message(&app.state.messages[app.state.current_index]);
            app.state.set_status(String::from("Committing..."));
            app.state.spinner = Some(SpinnerState::new());

            // Perform the commit
            match app.perform_commit(&commit_message) {
                Ok(()) => {
                    app.state.set_status(String::from("Commit successful!"));
                    InputResult::Exit // Exit the TUI after successful commit
                }
                Err(e) => {
                    app.state.set_status(format!("Commit failed: {}", e));
                    InputResult::Continue
                }
            }
        }
        KeyCode::Esc => InputResult::Exit,
        _ => InputResult::Continue,
    }
}

fn handle_editing_message(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state.messages[app.state.current_index] = crate::context::GeneratedMessage {
                emoji: Some(app.state.selected_emoji.clone()),
                title: app.state.message_textarea.lines().join("\n"),
                message: String::new(),
            };
            app.state
                .set_status(String::from("Commit message updated."));
            InputResult::Continue
        }
        _ => {
            app.state.message_textarea.input(key);
            InputResult::Continue
        }
    }
}

fn handle_editing_instructions(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state.custom_instructions = app.state.instructions_textarea.lines().join("\n");
            app.state.set_status(String::from("Instructions updated."));
            app.handle_regenerate();
            InputResult::Continue
        }
        _ => {
            app.state.instructions_textarea.input(key);
            InputResult::Continue
        }
    }
}

fn handle_selecting_emoji(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state
                .set_status(String::from("Emoji selection cancelled."));
            InputResult::Continue
        }
        KeyCode::Enter => {
            if let Some(selected) = app.state.emoji_list_state.selected() {
                if selected < app.state.emoji_list.len() {
                    let selected_emoji = &app.state.emoji_list[selected];
                    if selected_emoji.0 == "No Emoji" {
                        app.state.selected_emoji = String::new();
                    } else {
                        app.state.selected_emoji = selected_emoji.0.clone();
                    }
                    app.state.mode = Mode::Normal;
                    app.state
                        .set_status(format!("Emoji selected: {}", app.state.selected_emoji));
                }
            }
            InputResult::Continue
        }
        KeyCode::Up => {
            if !app.state.emoji_list.is_empty() {
                let selected = app.state.emoji_list_state.selected().unwrap_or(0);
                let new_selected = if selected > 0 {
                    selected - 1
                } else {
                    app.state.emoji_list.len() - 1
                };
                app.state.emoji_list_state.select(Some(new_selected));
            }
            InputResult::Continue
        }
        KeyCode::Down => {
            if !app.state.emoji_list.is_empty() {
                let selected = app.state.emoji_list_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % app.state.emoji_list.len();
                app.state.emoji_list_state.select(Some(new_selected));
            }
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn handle_selecting_preset(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state
                .set_status(String::from("Preset selection cancelled."));
            InputResult::Continue
        }
        KeyCode::Enter => {
            if let Some(selected) = app.state.preset_list_state.selected() {
                app.state.selected_preset = app.state.preset_list[selected].0.clone();
                app.state.mode = Mode::Normal;
                app.state.set_status(format!(
                    "Preset selected: {}",
                    app.state.get_selected_preset_name_with_emoji()
                ));
                app.handle_regenerate();
            }
            InputResult::Continue
        }
        KeyCode::Up => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = if selected > 0 {
                selected - 1
            } else {
                app.state.preset_list.len() - 1
            };
            app.state.preset_list_state.select(Some(new_selected));
            InputResult::Continue
        }
        KeyCode::Down => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = (selected + 1) % app.state.preset_list.len();
            app.state.preset_list_state.select(Some(new_selected));
            InputResult::Continue
        }
        KeyCode::PageUp => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = if selected > 10 { selected - 10 } else { 0 };
            app.state.preset_list_state.select(Some(new_selected));
            InputResult::Continue
        }
        KeyCode::PageDown => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = if selected + 10 < app.state.preset_list.len() {
                selected + 10
            } else {
                app.state.preset_list.len() - 1
            };
            app.state.preset_list_state.select(Some(new_selected));
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn handle_editing_user_info(app: &mut TuiCommit, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state
                .set_status(String::from("User info editing cancelled."));
            InputResult::Continue
        }
        KeyCode::Enter => {
            app.state.user_name = app.state.user_name_textarea.lines().join("\n");
            app.state.user_email = app.state.user_email_textarea.lines().join("\n");
            app.state.mode = Mode::Normal;
            app.state.set_status(String::from("User info updated."));
            InputResult::Continue
        }
        KeyCode::Tab => {
            app.state.user_info_focus = match app.state.user_info_focus {
                UserInfoFocus::Name => UserInfoFocus::Email,
                UserInfoFocus::Email => UserInfoFocus::Name,
            };
            InputResult::Continue
        }
        _ => {
            let input_handled = match app.state.user_info_focus {
                UserInfoFocus::Name => app.state.user_name_textarea.input(key),
                UserInfoFocus::Email => app.state.user_email_textarea.input(key),
            };
            if !input_handled {
                app.state
                    .set_status(String::from("Unhandled input in user info editing"));
            }
            InputResult::Continue
        }
    }
}

pub enum InputResult {
    Continue,
    Exit,
}
