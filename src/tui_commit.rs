use crate::context::{format_commit_message, GeneratedMessage};
use crate::gitmoji::get_gitmoji_list;
use crate::instruction_presets::{get_instruction_preset_library, list_presets_formatted};
//use crate::log_debug;
use crate::messages::{get_random_message, ColoredMessage};
use crate::{log_debug, ui::*};
use anyhow::{Error, Result};
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq)]
enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    EditingUserInfo,
    SelectingEmoji,
    SelectingPreset,
    Generating,
}

#[derive(PartialEq)]
enum UserInfoFocus {
    Name,
    Email,
}
struct SpinnerState {
    frames: Vec<char>,
    current_frame: usize,
    message: ColoredMessage,
}

impl SpinnerState {
    fn new() -> Self {
        Self {
            frames: vec!['✦', '✧', '✶', '✷', '✸', '✹', '✺', '✻', '✼', '✽'],
            current_frame: 0,
            message: get_random_message(),
        }
    }

    fn tick(&mut self) -> (String, String, Color, usize) {
        let frame = self.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        let spinner_with_space = format!("{} ", frame); // Add space after spinner
        let width = spinner_with_space.width() + self.message.text.width();
        (
            spinner_with_space,
            self.message.text.clone(),
            self.message.color,
            width,
        )
    }
}

pub struct TuiCommit {
    messages: Vec<GeneratedMessage>,
    current_index: usize,
    custom_instructions: String,
    status: String,
    selected_emoji: String,
    selected_preset: String,
    mode: Mode,
    message_textarea: TextArea<'static>,
    instructions_textarea: TextArea<'static>,
    emoji_list: Vec<(String, String)>,
    emoji_list_state: ListState,
    preset_list: Vec<(String, String, String, String)>,
    preset_list_state: ListState,
    user_name: String,
    user_email: String,
    user_name_textarea: TextArea<'static>,
    user_email_textarea: TextArea<'static>,
    user_info_focus: UserInfoFocus,
    spinner: Option<SpinnerState>,
    generate_message: Arc<dyn Fn(&str, &str) + Send + Sync>,
    perform_commit: Arc<dyn Fn(&str) -> Result<(), Error> + Send + Sync>,
    message_receiver: mpsc::Receiver<Result<GeneratedMessage, Error>>,
}

impl TuiCommit {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        preset: String,
        user_name: String,
        user_email: String,
        generate_message: Arc<dyn Fn(&str, &str) + Send + Sync>,
        perform_commit: Arc<dyn Fn(&str) -> Result<(), Error> + Send + Sync>,
        message_receiver: mpsc::Receiver<Result<GeneratedMessage, Error>>,
    ) -> Self {
        let mut message_textarea = TextArea::default();
        let messages = if initial_messages.is_empty() {
            vec![GeneratedMessage {
                emoji: None,
                title: String::new(),
                message: String::new(),
            }] // Ensure we always have at least one (empty) message
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
                    parts[0].to_string(), // key
                    parts[1].to_string(), // emoji
                    parts[2].to_string(), // name
                    parts[3].to_string(), // description
                )
            })
            .collect();

        let mut preset_list_state = ListState::default();
        preset_list_state.select(Some(0));

        let mut user_name_textarea = TextArea::default();
        user_name_textarea.insert_str(&user_name);
        let mut user_email_textarea = TextArea::default();
        user_email_textarea.insert_str(&user_email);

        TuiCommit {
            messages: messages,
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
            generate_message,
            perform_commit,
            message_receiver,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        // restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // If there was an error, print it
        if let Err(ref err) = result {
            println!("{:?}", err)
        }

        // Return the result
        result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            // Handle message generation
            if self.mode == Mode::Generating {
                match self.message_receiver.try_recv() {
                    Ok(result) => match result {
                        Ok(new_message) => {
                            self.messages.push(new_message);
                            self.current_index = self.messages.len() - 1;
                            self.update_message_textarea();
                            self.mode = Mode::Normal;
                            self.set_status(String::from("New message generated successfully!"));
                        }
                        Err(e) => {
                            self.mode = Mode::Normal;
                            self.set_status(format!("Failed to generate new message: {}", e));
                        }
                    },
                    Err(mpsc::error::TryRecvError::Empty) => {}
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        return Err(anyhow::anyhow!("Message channel disconnected"));
                    }
                }
            }

            // Handle user input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match self.mode {
                        Mode::Normal => {
                            if key.code == KeyCode::Esc {
                                return Ok(());
                            }
                            self.handle_normal_mode(key);
                        }
                        Mode::EditingMessage => self.handle_editing_message(key),
                        Mode::EditingInstructions => self.handle_editing_instructions(key),
                        Mode::SelectingEmoji => self.handle_selecting_emoji(key),
                        Mode::SelectingPreset => self.handle_selecting_preset(key),
                        Mode::EditingUserInfo => self.handle_editing_user_info(key),
                        Mode::Generating => {
                            // Optionally handle input during generation, e.g., allow cancellation
                            if key.code == KeyCode::Esc {
                                self.mode = Mode::Normal;
                                self.status = String::from("Message generation cancelled.");
                                self.spinner.take(); // Remove the spinner
                            }
                        }
                    }
                }
            }

            // Handle commit action
            if self.mode == Mode::Normal && self.status == "Committing..." {
                let commit_message = format_commit_message(&self.messages[self.current_index]);
                match (self.perform_commit)(&commit_message) {
                    Ok(()) => {
                        self.status = String::from("Commit successful!");
                    }
                    Err(e) => {
                        self.status = format!("Commit failed: {}", e);
                    }
                }
            }
        }
    }

    fn set_status(&mut self, new_status: String) {
        self.status = new_status;
        self.spinner = None; // Clear the spinner when setting a new status
    }

    fn update_message_textarea(&mut self) {
        let mut new_textarea = TextArea::default();
        new_textarea.insert_str(&format_commit_message(&self.messages[self.current_index]));
        self.message_textarea = new_textarea;
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('e') => {
                self.mode = Mode::EditingMessage;
                self.status = String::from("Editing commit message. Press Esc to finish.");
            }
            KeyCode::Char('i') => {
                self.mode = Mode::EditingInstructions;
                self.status = String::from("Editing instructions. Press Esc to finish.");
            }
            KeyCode::Char('g') => {
                self.mode = Mode::SelectingEmoji;
                self.status = String::from(
                    "Selecting emoji. Use arrow keys and Enter to select, Esc to cancel.",
                );
            }
            KeyCode::Char('p') => {
                self.mode = Mode::SelectingPreset;
                self.status = String::from(
                    "Selecting preset. Use arrow keys and Enter to select, Esc to cancel.",
                );
            }
            KeyCode::Char('u') => {
                self.mode = Mode::EditingUserInfo;
                self.status = String::from(
                    "Editing user info. Press Tab to switch fields, Enter to save, Esc to cancel.",
                );
            }
            KeyCode::Char('r') => {
                self.mode = Mode::Generating;
                log_debug!(
                    ">>> Generating new message: instructions={} preset={}",
                    self.custom_instructions,
                    self.selected_preset
                );
                self.handle_regenerate();
            }
            KeyCode::Left => {
                if self.current_index > 0 {
                    self.current_index -= 1;
                } else {
                    self.current_index = self.messages.len() - 1;
                }
                self.update_message_textarea();
                self.status = format!(
                    "Viewing commit message {}/{}",
                    self.current_index + 1,
                    self.messages.len()
                );
            }
            KeyCode::Right => {
                if self.current_index < self.messages.len() - 1 {
                    self.current_index += 1;
                } else {
                    self.current_index = 0;
                }
                self.update_message_textarea();
                self.status = format!(
                    "Viewing commit message {}/{}",
                    self.current_index + 1,
                    self.messages.len()
                );
            }
            KeyCode::Enter => {
                let commit_message = format_commit_message(&self.messages[self.current_index]);
                if let Err(e) = (self.perform_commit)(&commit_message) {
                    self.status = format!("Failed to commit: {}", e);
                } else {
                    self.status = String::from("Commit successful!");
                }
            }
            _ => {}
        }
    }

    fn handle_editing_user_info(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.status = String::from("User info editing cancelled.");
            }
            KeyCode::Enter => {
                self.user_name = self.user_name_textarea.lines().join("\n");
                self.user_email = self.user_email_textarea.lines().join("\n");
                self.mode = Mode::Normal;
                self.status = String::from("User info updated.");
            }
            KeyCode::Tab => {
                self.user_info_focus = match self.user_info_focus {
                    UserInfoFocus::Name => UserInfoFocus::Email,
                    UserInfoFocus::Email => UserInfoFocus::Name,
                };
            }
            _ => {
                let input_handled = match self.user_info_focus {
                    UserInfoFocus::Name => self.user_name_textarea.input(key),
                    UserInfoFocus::Email => self.user_email_textarea.input(key),
                };
                if !input_handled {
                    self.status = String::from("Unhandled input in user info editing");
                }
            }
        }
    }

    fn handle_editing_message(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.messages[self.current_index] = GeneratedMessage {
                    emoji: Some(self.selected_emoji.clone()),
                    title: self.message_textarea.lines().join("\n"),
                    message: String::new(),
                };
                self.status = String::from("Commit message updated.");
            }
            _ => {
                self.message_textarea.input(key);
            }
        }
    }

    fn handle_editing_instructions(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.custom_instructions = self.instructions_textarea.lines().join("\n");
                self.status = String::from("Instructions updated.");
                // Regenerate message with new instructions
                self.handle_regenerate();
            }
            _ => {
                self.instructions_textarea.input(key);
            }
        }
    }

    fn handle_selecting_emoji(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.status = String::from("Emoji selection cancelled.");
            }
            KeyCode::Enter => {
                if let Some(selected) = self.emoji_list_state.selected() {
                    if selected < self.emoji_list.len() {
                        let selected_emoji = &self.emoji_list[selected];
                        if selected_emoji.0 == "No Emoji" {
                            self.selected_emoji = String::new();
                        } else {
                            self.selected_emoji = selected_emoji.0.clone();
                        }
                        self.mode = Mode::Normal;
                        self.status = format!("Emoji selected: {}", self.selected_emoji);
                    }
                }
            }
            KeyCode::Up => {
                if !self.emoji_list.is_empty() {
                    let selected = self.emoji_list_state.selected().unwrap_or(0);
                    let new_selected = if selected > 0 {
                        selected - 1
                    } else {
                        self.emoji_list.len() - 1
                    };
                    self.emoji_list_state.select(Some(new_selected));
                }
            }
            KeyCode::Down => {
                if !self.emoji_list.is_empty() {
                    let selected = self.emoji_list_state.selected().unwrap_or(0);
                    let new_selected = (selected + 1) % self.emoji_list.len();
                    self.emoji_list_state.select(Some(new_selected));
                }
            }
            _ => {}
        }
    }

    fn handle_regenerate(&mut self) {
        self.mode = Mode::Generating;
        self.spinner = Some(SpinnerState::new());
        (self.generate_message)(&self.selected_preset, &self.custom_instructions);
    }

    fn handle_selecting_preset(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.status = String::from("Preset selection cancelled.");
            }
            KeyCode::Enter => {
                if let Some(selected) = self.preset_list_state.selected() {
                    self.selected_preset = self.preset_list[selected].0.clone();
                    self.mode = Mode::Normal;
                    self.status = format!(
                        "Preset selected: {}",
                        self.get_selected_preset_name_with_emoji()
                    );
                    // Regenerate message with new preset
                    self.handle_regenerate();
                }
            }
            KeyCode::Up => {
                let selected = self.preset_list_state.selected().unwrap_or(0);
                let new_selected = if selected > 0 {
                    selected - 1
                } else {
                    self.preset_list.len() - 1
                };
                self.preset_list_state.select(Some(new_selected));
            }
            KeyCode::Down => {
                let selected = self.preset_list_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % self.preset_list.len();
                self.preset_list_state.select(Some(new_selected));
            }
            KeyCode::PageUp => {
                let selected = self.preset_list_state.selected().unwrap_or(0);
                let new_selected = if selected > 10 { selected - 10 } else { 0 };
                self.preset_list_state.select(Some(new_selected));
            }
            KeyCode::PageDown => {
                let selected = self.preset_list_state.selected().unwrap_or(0);
                let new_selected = if selected + 10 < self.preset_list.len() {
                    selected + 10
                } else {
                    self.preset_list.len() - 1
                };
                self.preset_list_state.select(Some(new_selected));
            }
            _ => {}
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Title
                    Constraint::Length(2), // Navigation bar
                    Constraint::Length(2), // User info
                    Constraint::Min(5),    // Commit message
                    Constraint::Length(8), // Instructions
                    Constraint::Length(3), // Emoji and Preset
                    Constraint::Length(1), // Status
                ]
                .as_ref(),
            )
            .split(f.size());

        // Title
        let title = vec![
            Line::from(Span::styled(
                "    .  *  .  ✨  .  *  .    ",
                Style::default().fg(GALAXY_PINK),
            )),
            Line::from(vec![
                Span::styled("  *    ", Style::default().fg(GALAXY_PINK)),
                Span::styled(
                    "✨🔮 Git-Iris v0.1.0 - Cosmic Commit 🔮✨",
                    Style::default()
                        .fg(GALAXY_PINK)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("   * ", Style::default().fg(GALAXY_PINK)),
            ]),
        ];

        let title_widget = Paragraph::new(title).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(title_widget, chunks[0]);

        // Navigation bar
        let nav_items = vec![
            ("←→", "Navigate", CELESTIAL_BLUE),
            ("E", "Message", SOLAR_YELLOW),
            ("I", "Instructions", AURORA_GREEN),
            ("G", "Emoji", PLASMA_CYAN),
            ("P", "Preset", COMET_ORANGE),
            ("U", "User Info", GALAXY_PINK),
            ("R", "Regenerate", METEOR_RED),
            ("⏎", "Commit", STARLIGHT),
        ];
        let nav_spans: Vec<Span> = nav_items
            .into_iter()
            .flat_map(|(key, desc, color)| {
                vec![
                    Span::styled(
                        format!("{}", key),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(": {} ", desc), Style::default().fg(NEBULA_PURPLE)),
                ]
            })
            .collect();
        let nav_bar =
            Paragraph::new(Line::from(nav_spans)).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(nav_bar, chunks[1]);

        // User info
        let user_info = Paragraph::new(Line::from(vec![
            Span::styled("👤 ", Style::default().fg(PLASMA_CYAN)),
            Span::styled(
                &self.user_name,
                Style::default()
                    .fg(AURORA_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled("✉️ ", Style::default().fg(PLASMA_CYAN)),
            Span::styled(
                &self.user_email,
                Style::default()
                    .fg(AURORA_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(user_info, chunks[2]);

        // Commit message
        let message_title = format!(
            "✦ Commit Message ({}/{})",
            self.current_index + 1,
            self.messages.len()
        );
        let message_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CELESTIAL_BLUE))
            .title(Span::styled(
                message_title,
                Style::default()
                    .fg(GALAXY_PINK)
                    .add_modifier(Modifier::BOLD),
            ));

        let message_content = if self.mode == Mode::EditingMessage {
            self.message_textarea.lines().join("\n")
        } else {
            format_commit_message(&self.messages[self.current_index])
        };

        let message = Paragraph::new(message_content)
            .block(message_block)
            .style(Style::default().fg(SOLAR_YELLOW))
            .wrap(Wrap { trim: true });

        f.render_widget(message, chunks[3]);

        // Instructions
        let instructions_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CELESTIAL_BLUE))
            .title(Span::styled(
                "✧ Custom Instructions",
                Style::default()
                    .fg(GALAXY_PINK)
                    .add_modifier(Modifier::BOLD),
            ));

        match self.mode {
            Mode::EditingInstructions => {
                self.instructions_textarea.set_block(instructions_block);
                self.instructions_textarea
                    .set_style(Style::default().fg(PLASMA_CYAN));
                f.render_widget(&self.instructions_textarea, chunks[4]);
            }
            _ => {
                let instructions = Paragraph::new(self.custom_instructions.clone())
                    .block(instructions_block)
                    .style(Style::default().fg(PLASMA_CYAN))
                    .wrap(Wrap { trim: true });
                f.render_widget(instructions, chunks[4]);
            }
        }

        // Emoji and Preset
        let binding = self.get_selected_preset_name_with_emoji();
        let emoji_preset = Paragraph::new(Line::from(vec![
            Span::styled("Emoji: ", Style::default().fg(NEBULA_PURPLE)),
            Span::styled(
                &self.selected_emoji,
                Style::default()
                    .fg(SOLAR_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  "),
            Span::styled("Preset: ", Style::default().fg(NEBULA_PURPLE)),
            Span::styled(
                &binding,
                Style::default()
                    .fg(COMET_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(emoji_preset, chunks[5]);

        // Status
        let (spinner_with_space, status_content, color, content_width) =
            if let Some(spinner) = &mut self.spinner {
                spinner.tick()
            } else {
                (
                    "  ".to_string(),
                    self.status.clone(),
                    AURORA_GREEN,
                    self.status.width() + 2,
                )
            };

        let terminal_width = f.size().width as usize;
        let left_padding = (terminal_width - content_width) / 2;
        let right_padding = terminal_width - content_width - left_padding;

        let status_line = Line::from(vec![
            Span::raw(" ".repeat(left_padding)),
            Span::styled(spinner_with_space, Style::default().fg(PLASMA_CYAN)),
            Span::styled(status_content, Style::default().fg(color)),
            Span::raw(" ".repeat(right_padding)),
        ]);

        let status_widget =
            Paragraph::new(vec![status_line]).alignment(ratatui::layout::Alignment::Left);
        f.render_widget(Clear, chunks[6]); // Clear the entire status line
        f.render_widget(status_widget, chunks[6]);

        if self.mode == Mode::SelectingEmoji {
            self.render_emoji_popup(f);
        } else if self.mode == Mode::SelectingPreset {
            self.render_preset_popup(f);
        } else if self.mode == Mode::EditingUserInfo {
            self.render_user_info_popup(f);
        }
    }

    fn render_emoji_popup(&mut self, f: &mut Frame) {
        let popup_block = Block::default()
            .title(Span::styled(
                "✨ Select Emoji",
                Style::default()
                    .fg(SOLAR_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEBULA_PURPLE));

        let area = f.size();
        let popup_area = Rect::new(
            area.x + 10,
            area.y + 5,
            area.width.saturating_sub(20).min(60),
            area.height.saturating_sub(10).min(20),
        );

        let items: Vec<ListItem> = self
            .emoji_list
            .iter()
            .map(|(emoji, description)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", emoji), Style::default().fg(SOLAR_YELLOW)),
                    Span::styled(description, Style::default().fg(PLASMA_CYAN)),
                ]))
            })
            .collect();

        let list = List::new(items).block(popup_block).highlight_style(
            Style::default()
                .bg(CELESTIAL_BLUE)
                .fg(STARLIGHT)
                .add_modifier(Modifier::BOLD),
        );

        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut self.emoji_list_state);
    }

    fn render_preset_popup(&mut self, f: &mut Frame) {
        let popup_block = Block::default()
            .title(Span::styled(
                "🌟 Select Preset",
                Style::default()
                    .fg(COMET_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEBULA_PURPLE));

        let area = f.size();
        let popup_area = Rect::new(
            area.x + 5,
            area.y + 5,
            area.width.saturating_sub(10).min(70),
            area.height.saturating_sub(10).min(20),
        );

        let items: Vec<ListItem> = self
            .preset_list
            .iter()
            .map(|(_, emoji, name, description)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} {} ", emoji, name),
                        Style::default().fg(COMET_ORANGE),
                    ),
                    Span::styled(description, Style::default().fg(PLASMA_CYAN)),
                ]))
            })
            .collect();

        let list = List::new(items).block(popup_block).highlight_style(
            Style::default()
                .bg(CELESTIAL_BLUE)
                .fg(STARLIGHT)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut self.preset_list_state);
    }

    fn get_selected_preset_name_with_emoji(&self) -> String {
        self.preset_list
            .iter()
            .find(|(key, _, _, _)| key == &self.selected_preset)
            .map(|(_, emoji, name, _)| format!("{} {}", emoji, name))
            .unwrap_or_else(|| "None".to_string())
    }

    fn render_user_info_popup(&mut self, f: &mut Frame) {
        let popup_block = Block::default()
            .title(Span::styled(
                "Edit User Info",
                Style::default()
                    .fg(SOLAR_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEBULA_PURPLE));

        let area = f.size();
        let popup_area = Rect::new(
            area.x + 10,
            area.y + 5,
            area.width.saturating_sub(20).min(60),
            area.height.saturating_sub(10).min(10),
        );

        let popup_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Name
                    Constraint::Length(3), // Email
                ]
                .as_ref(),
            )
            .split(popup_area);

        self.user_name_textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(if self.user_info_focus == UserInfoFocus::Name {
                        SOLAR_YELLOW
                    } else {
                        CELESTIAL_BLUE
                    }),
                )
                .title("Name"),
        );

        self.user_email_textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(
                    if self.user_info_focus == UserInfoFocus::Email {
                        SOLAR_YELLOW
                    } else {
                        CELESTIAL_BLUE
                    },
                ))
                .title("Email"),
        );

        f.render_widget(Clear, popup_area);
        f.render_widget(popup_block, popup_area);
        f.render_widget(&self.user_name_textarea, popup_chunks[0]);
        f.render_widget(&self.user_email_textarea, popup_chunks[1]);
    }
}

pub fn run_tui_commit(
    initial_messages: Vec<GeneratedMessage>,
    custom_instructions: String,
    selected_preset: String,
    user_name: String,
    user_email: String,
    generate_message: Arc<dyn Fn(&str, &str) + Send + Sync>,
    perform_commit: Arc<dyn Fn(&str) -> Result<(), Error> + Send + Sync>,
    message_receiver: mpsc::Receiver<Result<GeneratedMessage, Error>>,
) -> Result<()> {
    let mut app = TuiCommit::new(
        initial_messages,
        custom_instructions,
        selected_preset,
        user_name,
        user_email,
        generate_message,
        perform_commit,
        message_receiver,
    );
    app.run().map_err(Error::from)
}

/*
fn main() -> Result<()> {
    let initial_messages = vec![String::from("feat: Implement cosmic TUI for Git-Iris")];
    let custom_instructions = String::from("Channel the cosmic energy to craft a commit message that aligns with the celestial Conventional Commits specification. Focus on the main changes and their impact on the cosmic codebase.");
    let user_name = String::from("Stefanie Jane");
    let user_email = String::from("stef@hyperbliss.tech");

    // These are placeholder implementations. In a real application, these would interact with your LLM and Git systems.
    let generate_message = || -> Result<String, Error> {
        Ok(String::from("feat: Add new cosmic feature to Git-Iris"))
    };

    let perform_commit = |message: &str| -> Result<(), Error> {
        println!("Committing message: {}", message);
        Ok(())
    };

    run_tui_commit(
        initial_messages,
        custom_instructions,
        user_name,
        user_email,
        generate_message,
        perform_commit,
    )
}
    */
