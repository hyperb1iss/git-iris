use super::app::TuiCommit;
use super::state::{Mode, UserInfoFocus};
use crate::context::format_commit_message;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

pub fn handle_input(app: &mut TuiCommit, key: KeyEvent) {
    match app.state.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::EditingMessage => handle_editing_message(app, key),
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
        }
    }
}

fn handle_normal_mode(app: &mut TuiCommit, key: KeyEvent) {
    match key.code {
        KeyCode::Char('e') => {
            app.state.mode = Mode::EditingMessage;
            app.state
                .set_status(String::from("Editing commit message. Press Esc to finish."));
        }
        KeyCode::Char('i') => {
            app.state.mode = Mode::EditingInstructions;
            app.state
                .set_status(String::from("Editing instructions. Press Esc to finish."));
        }
        KeyCode::Char('g') => {
            app.state.mode = Mode::SelectingEmoji;
            app.state.set_status(String::from(
                "Selecting emoji. Use arrow keys and Enter to select, Esc to cancel.",
            ));
        }
        KeyCode::Char('p') => {
            app.state.mode = Mode::SelectingPreset;
            app.state.set_status(String::from(
                "Selecting preset. Use arrow keys and Enter to select, Esc to cancel.",
            ));
        }
        KeyCode::Char('u') => {
            app.state.mode = Mode::EditingUserInfo;
            app.state.set_status(String::from(
                "Editing user info. Press Tab to switch fields, Enter to save, Esc to cancel.",
            ));
        }
        KeyCode::Char('r') => {
            app.handle_regenerate();
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
        }
        KeyCode::Enter => {
            let commit_message =
                format_commit_message(&app.state.messages[app.state.current_index]);
            app.state.set_status(String::from("Committing..."));
        }
        _ => {}
    }
}

fn handle_editing_message(app: &mut TuiCommit, key: KeyEvent) {
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
        }
        _ => {
            app.state.message_textarea.input(key);
        }
    }
}

fn handle_editing_instructions(app: &mut TuiCommit, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state.custom_instructions = app.state.instructions_textarea.lines().join("\n");
            app.state.set_status(String::from("Instructions updated."));
            app.handle_regenerate();
        }
        _ => {
            app.state.instructions_textarea.input(key);
        }
    }
}

fn handle_selecting_emoji(app: &mut TuiCommit, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state
                .set_status(String::from("Emoji selection cancelled."));
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
        }
        KeyCode::Down => {
            if !app.state.emoji_list.is_empty() {
                let selected = app.state.emoji_list_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % app.state.emoji_list.len();
                app.state.emoji_list_state.select(Some(new_selected));
            }
        }
        _ => {}
    }
}

fn handle_selecting_preset(app: &mut TuiCommit, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state
                .set_status(String::from("Preset selection cancelled."));
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
        }
        KeyCode::Up => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = if selected > 0 {
                selected - 1
            } else {
                app.state.preset_list.len() - 1
            };
            app.state.preset_list_state.select(Some(new_selected));
        }
        KeyCode::Down => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = (selected + 1) % app.state.preset_list.len();
            app.state.preset_list_state.select(Some(new_selected));
        }
        KeyCode::PageUp => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = if selected > 10 { selected - 10 } else { 0 };
            app.state.preset_list_state.select(Some(new_selected));
        }
        KeyCode::PageDown => {
            let selected = app.state.preset_list_state.selected().unwrap_or(0);
            let new_selected = if selected + 10 < app.state.preset_list.len() {
                selected + 10
            } else {
                app.state.preset_list.len() - 1
            };
            app.state.preset_list_state.select(Some(new_selected));
        }
        _ => {}
    }
}

fn handle_editing_user_info(app: &mut TuiCommit, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.state.mode = Mode::Normal;
            app.state
                .set_status(String::from("User info editing cancelled."));
        }
        KeyCode::Enter => {
            app.state.user_name = app.state.user_name_textarea.lines().join("\n");
            app.state.user_email = app.state.user_email_textarea.lines().join("\n");
            app.state.mode = Mode::Normal;
            app.state.set_status(String::from("User info updated."));
        }
        KeyCode::Tab => {
            app.state.user_info_focus = match app.state.user_info_focus {
                UserInfoFocus::Name => UserInfoFocus::Email,
                UserInfoFocus::Email => UserInfoFocus::Name,
            };
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
        }
    }
}
