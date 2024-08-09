use super::app::TuiCommit;
use super::spinner::SpinnerState;
use super::state::{EmojiMode, Mode, UserInfoFocus};
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
            let edited_content = app.state.message_textarea.lines().join("\n");
            if let Some(message) = app.state.messages.get_mut(app.state.current_index) {
                // Split the edited content into title and message
                let mut lines = edited_content.lines();
                let title_line = lines.next().unwrap_or("").trim();

                // Extract emoji if present at the start of the title
                let (emoji, title) = if let Some(first_char) = title_line.chars().next() {
                    if is_emoji(first_char) {
                        let (emoji, rest) = title_line.split_at(first_char.len_utf8());
                        (Some(emoji.to_string()), rest.trim().to_string())
                    } else {
                        (None, title_line.to_string())
                    }
                } else {
                    (None, title_line.to_string())
                };

                // Update message fields
                message.emoji = emoji;
                message.title = title;

                // Collect the rest of the lines, skipping any leading empty lines
                message.message = lines
                    .skip_while(|line| line.trim().is_empty())
                    .collect::<Vec<&str>>()
                    .join("\n");
            }
            app.state
                .set_status(String::from("Commit message updated."));
            app.state.update_message_textarea();
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
                let new_emoji_mode = match selected {
                    0 => EmojiMode::None,
                    1 => EmojiMode::Auto,
                    _ => EmojiMode::Custom(app.state.emoji_list[selected].0.clone()),
                };

                // Apply the selected emoji to the current message
                if let Some(message) = app.state.messages.get_mut(app.state.current_index) {
                    message.emoji = match &new_emoji_mode {
                        EmojiMode::None => None,
                        EmojiMode::Auto => message.emoji.clone(), // Keep existing emoji
                        EmojiMode::Custom(emoji) => Some(emoji.clone()),
                    };
                }

                app.state.emoji_mode = new_emoji_mode;
                app.state.mode = Mode::Normal;
                app.state
                    .set_status(format!("Emoji mode set to: {:?}", app.state.emoji_mode));
                app.state.update_message_textarea();
            }
            InputResult::Continue
        }
        KeyCode::Up => {
            if let Some(selected) = app.state.emoji_list_state.selected() {
                let new_selection = if selected > 0 {
                    selected - 1
                } else {
                    app.state.emoji_list.len() - 1
                };
                app.state.emoji_list_state.select(Some(new_selection));
            }
            InputResult::Continue
        }
        KeyCode::Down => {
            if let Some(selected) = app.state.emoji_list_state.selected() {
                let new_selection = (selected + 1) % app.state.emoji_list.len();
                app.state.emoji_list_state.select(Some(new_selection));
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

fn is_emoji(c: char) -> bool {
    matches!(c,
        '\u{1F300}'..='\u{1F5FF}' | '\u{1F900}'..='\u{1F9FF}' |
        '\u{1F600}'..='\u{1F64F}' | '\u{1FA70}'..='\u{1FAFF}' |
        '\u{2600}'..='\u{26FF}' | '\u{2700}'..='\u{27BF}' |
        '\u{1F680}'..='\u{1F6FF}'
    )
}

pub enum InputResult {
    Continue,
    Exit,
}
