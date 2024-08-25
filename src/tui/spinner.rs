use crate::messages::get_waiting_message;
use crate::messages::ColoredMessage;
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

pub struct SpinnerState {
    frames: Vec<char>,
    current_frame: usize,
    message: ColoredMessage,
}

impl SpinnerState {
    pub fn new() -> Self {
        Self {
            frames: vec!['✦', '✧', '✶', '✷', '✸', '✹', '✺', '✻', '✼', '✽'],
            current_frame: 0,
            message: get_waiting_message(),
        }
    }

    pub fn tick(&mut self) -> (String, String, Color, usize) {
        let frame = self.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        let spinner_with_space = format!("{} ", frame);
        let width = spinner_with_space.width() + self.message.text.width();
        (
            spinner_with_space,
            self.message.text.clone(),
            self.message.color,
            width,
        )
    }
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self::new()
    }
}
