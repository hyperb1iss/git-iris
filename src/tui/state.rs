use crate::context::{format_commit_message, GeneratedMessage};
use crate::gitmoji::get_gitmoji_list;
use crate::instruction_presets::{get_instruction_preset_library, list_presets_formatted};
use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use super::spinner::SpinnerState;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    EditingUserInfo,
    SelectingEmoji,
    SelectingPreset,
    Generating,
}

#[derive(PartialEq)]
pub enum UserInfoFocus {
    Name,
    Email,
}

pub struct TuiState {
    pub messages: Vec<GeneratedMessage>,
    pub current_index: usize,
    pub custom_instructions: String,
    pub status: String,
    pub selected_emoji: String,
    pub selected_preset: String,
    pub mode: Mode,
    pub message_textarea: TextArea<'static>,
    pub instructions_textarea: TextArea<'static>,
    pub emoji_list: Vec<(String, String)>,
    pub emoji_list_state: ListState,
    pub preset_list: Vec<(String, String, String, String)>,
    pub preset_list_state: ListState,
    pub user_name: String,
    pub user_email: String,
    pub user_name_textarea: TextArea<'static>,
    pub user_email_textarea: TextArea<'static>,
    pub user_info_focus: UserInfoFocus,
    pub spinner: Option<SpinnerState>,
    pub dirty: bool, // Used to track if we need to redraw
    pub last_spinner_update: std::time::Instant,
}

impl TuiState {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        preset: String,
        user_name: String,
        user_email: String,
    ) -> Self {
        let mut message_textarea = TextArea::default();
        let messages = if initial_messages.is_empty() {
            vec![GeneratedMessage {
                emoji: None,
                title: String::new(),
                message: String::new(),
            }]
        } else {
            initial_messages
        };
        if let Some(first_message) = messages.first() {
            message_textarea.insert_str(&format_commit_message(first_message));
        }

        let mut instructions_textarea = TextArea::default();
        instructions_textarea.insert_str(&custom_instructions);

        let emoji_list: Vec<_> = get_gitmoji_list()
            .split('\n')
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                (parts[0].to_string(), parts[1].to_string())
            })
            .collect();

        let mut emoji_list_state = ListState::default();
        if !emoji_list.is_empty() {
            emoji_list_state.select(Some(0));
        }

        let preset_library = get_instruction_preset_library();
        let preset_list = list_presets_formatted(&preset_library)
            .split('\n')
            .map(|line| {
                let parts: Vec<&str> = line.split(" - ").collect();
                (
                    parts[0].to_string(),
                    parts[1].to_string(),
                    parts[2].to_string(),
                    parts[3].to_string(),
                )
            })
            .collect();

        let mut preset_list_state = ListState::default();
        preset_list_state.select(Some(0));

        let mut user_name_textarea = TextArea::default();
        user_name_textarea.insert_str(&user_name);
        let mut user_email_textarea = TextArea::default();
        user_email_textarea.insert_str(&user_email);

        TuiState {
            messages,
            current_index: 0,
            custom_instructions,
            status: String::from("🌌 Cosmic energies aligning. Press 'Esc' to exit."),
            selected_emoji: String::from("✨"),
            selected_preset: preset,
            mode: Mode::Normal,
            message_textarea,
            instructions_textarea,
            emoji_list,
            emoji_list_state,
            preset_list,
            preset_list_state,
            user_name,
            user_email,
            user_name_textarea,
            user_email_textarea,
            user_info_focus: UserInfoFocus::Name,
            spinner: None,
            dirty: true,
            last_spinner_update: std::time::Instant::now(),
        }
    }

    pub fn set_status(&mut self, new_status: String) {
        self.status = new_status;
        self.spinner = None;
        self.dirty = true;
    }

    pub fn update_message_textarea(&mut self) {
        let mut new_textarea = TextArea::default();
        new_textarea.insert_str(&format_commit_message(&self.messages[self.current_index]));
        self.message_textarea = new_textarea;
        self.dirty = true;
    }

    pub fn get_selected_preset_name_with_emoji(&self) -> String {
        self.preset_list
            .iter()
            .find(|(key, _, _, _)| key == &self.selected_preset)
            .map(|(_, emoji, name, _)| format!("{} {}", emoji, name))
            .unwrap_or_else(|| "None".to_string())
    }
}
