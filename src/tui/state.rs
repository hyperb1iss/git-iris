use crate::context::{format_commit_message, GeneratedMessage};
use crate::gitmoji::get_gitmoji_list;
use crate::instruction_presets::{get_instruction_preset_library, list_presets_formatted};
use crate::messages::get_user_message;
use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use super::spinner::SpinnerState;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    EditingUserInfo,
    SelectingEmoji,
    SelectingPreset,
    Generating,
}

#[derive(PartialEq, Eq)]
pub enum UserInfoFocus {
    Name,
    Email,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmojiMode {
    None,
    Auto,
    Custom(String),
}

pub struct TuiState {
    pub messages: Vec<GeneratedMessage>,
    pub current_index: usize,
    pub custom_instructions: String,
    pub status: String,
    pub selected_emoji: Option<String>,
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
    pub emoji_mode: EmojiMode,
}

impl TuiState {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        preset: String,
        user_name: String,
        user_email: String,
        use_gitmoji: bool,
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
            message_textarea.insert_str(format_commit_message(first_message));
        }

        let mut instructions_textarea = TextArea::default();
        instructions_textarea.insert_str(&custom_instructions);

        let mut emoji_list = vec![
            ("None".to_string(), "No emoji".to_string()),
            ("Auto".to_string(), "Let AI choose".to_string()),
        ];
        emoji_list.extend(get_gitmoji_list().split('\n').map(|line| {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            (parts[0].to_string(), parts[1].to_string())
        }));

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

        Self {
            messages,
            current_index: 0,
            custom_instructions,
            status: format!("{}.. Press 'Esc' to exit.", get_user_message().text),
            selected_emoji: None,
            selected_preset: preset,
            mode: Mode::Normal,
            message_textarea,
            instructions_textarea,
            emoji_list,
            emoji_list_state,
            emoji_mode: if use_gitmoji {
                EmojiMode::Auto
            } else {
                EmojiMode::None
            },
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
        let current_message = &self.messages[self.current_index];
        let emoji_prefix = self
            .get_current_emoji()
            .map_or(String::new(), |e| format!("{e} "));
        let message_content = format!(
            "{}{}\n\n{}",
            emoji_prefix,
            current_message.title,
            current_message.message.trim()
        );

        let mut new_textarea = TextArea::default();
        new_textarea.insert_str(&message_content);
        self.message_textarea = new_textarea;
        self.dirty = true;
    }

    pub fn get_selected_preset_name_with_emoji(&self) -> String {
        self.preset_list
            .iter()
            .find(|(key, _, _, _)| key == &self.selected_preset)
            .map_or_else(
                || "None".to_string(),
                |(_, emoji, name, _)| format!("{emoji} {name}"),
            )
    }

    pub fn get_current_emoji(&self) -> Option<String> {
        match &self.emoji_mode {
            EmojiMode::None => None,
            EmojiMode::Auto => self.messages[self.current_index].emoji.clone(),
            EmojiMode::Custom(emoji) => Some(emoji.clone()),
        }
    }

    pub fn apply_selected_emoji(&mut self) {
        if let Some(message) = self.messages.get_mut(self.current_index) {
            message.emoji.clone_from(&self.selected_emoji);
        }
    }
}
