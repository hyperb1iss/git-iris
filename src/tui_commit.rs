use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
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
use std::time::Duration;
use tui_textarea::TextArea;

mod gitmoji;
use gitmoji::get_gitmoji_list;
mod instruction_presets;
use instruction_presets::{get_instruction_preset_library, list_presets_formatted};

// Cosmic color palette
const STARLIGHT: Color = Color::Rgb(255, 255, 240);
const NEBULA_PURPLE: Color = Color::Rgb(167, 132, 239);
const CELESTIAL_BLUE: Color = Color::Rgb(75, 115, 235);
const SOLAR_YELLOW: Color = Color::Rgb(255, 225, 100);
const AURORA_GREEN: Color = Color::Rgb(140, 255, 170);
const PLASMA_CYAN: Color = Color::Rgb(20, 255, 255);
const METEOR_RED: Color = Color::Rgb(255, 89, 70);
const GALAXY_PINK: Color = Color::Rgb(255, 162, 213);
const COMET_ORANGE: Color = Color::Rgb(255, 165, 0);

#[derive(PartialEq)]
enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    EditingUserInfo,
    SelectingEmoji,
    SelectingPreset,
}

#[derive(PartialEq)]
enum UserInfoFocus {
    Name,
    Email,
}

pub struct TuiCommit {
    messages: Vec<String>,
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
}

impl TuiCommit {
    pub fn new(initial_message: String, custom_instructions: String) -> Self {
        let mut message_textarea = TextArea::default();
        message_textarea.insert_str(&initial_message);
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

        let mut preset_list_state = ListState::default();
        preset_list_state.select(Some(0));

        let mut user_name_textarea = TextArea::default();
        user_name_textarea.insert_str("Stefanie Jane");
        let mut user_email_textarea = TextArea::default();
        user_email_textarea.insert_str("stef@hyperbliss.tech");

        TuiCommit {
            messages: vec![initial_message],
            current_index: 0,
            custom_instructions,
            status: String::from("🌌 Cosmic energies aligning. Press 'Esc' to exit."),
            selected_emoji: String::from("✨"),
            selected_preset: String::from("default"),
            mode: Mode::Normal,
            message_textarea,
            instructions_textarea,
            emoji_list,
            emoji_list_state,
            preset_list,
            preset_list_state,
            user_name: String::from("Stefanie Jane"),
            user_email: String::from("stef@hyperbliss.tech"),
            user_name_textarea,
            user_email_textarea,
            user_info_focus: UserInfoFocus::Name,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal);

        // restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    }

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                        return Ok(());
                    }
                    match self.mode {
                        Mode::Normal => {
                            if key.code == KeyCode::Esc {
                                return Ok(());
                            }
                            self.handle_normal_mode(key)
                        }
                        Mode::EditingMessage => self.handle_editing_message(key),
                        Mode::EditingInstructions => self.handle_editing_instructions(key),
                        Mode::SelectingEmoji => self.handle_selecting_emoji(key),
                        Mode::SelectingPreset => self.handle_selecting_preset(key),
                        Mode::EditingUserInfo => self.handle_editing_user_info(key),
                    }
                }
            }
        }
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
            KeyCode::Left => {
                if self.current_index > 0 {
                    self.current_index -= 1;
                    self.status = format!(
                        "Viewing commit message {}/{}",
                        self.current_index + 1,
                        self.messages.len()
                    );
                }
            }
            KeyCode::Right => {
                if self.current_index < self.messages.len() - 1 {
                    self.current_index += 1;
                    self.status = format!(
                        "Viewing commit message {}/{}",
                        self.current_index + 1,
                        self.messages.len()
                    );
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
                    // If the input wasn't handled by the textarea, you can add custom handling here
                    // For now, we'll just update the status
                    self.status = String::from("Unhandled input in user info editing");
                }
            }
        }
    }

    fn handle_editing_message(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.messages[self.current_index] = self.message_textarea.lines().join("\n");
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
            KeyCode::Left => {
                // Handle horizontal scrolling left
                // Implement horizontal scrolling logic here
            }
            KeyCode::Right => {
                // Handle horizontal scrolling right
                // Implement horizontal scrolling logic here
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
                    Constraint::Length(5), // Instructions
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
        match self.mode {
            Mode::EditingMessage => {
                self.message_textarea.set_block(message_block);
                self.message_textarea
                    .set_style(Style::default().fg(SOLAR_YELLOW));
                f.render_widget(&self.message_textarea, chunks[3]);
            }
            _ => {
                let message = Paragraph::new(self.messages[self.current_index].clone())
                    .block(message_block)
                    .style(Style::default().fg(SOLAR_YELLOW))
                    .wrap(Wrap { trim: true });
                f.render_widget(message, chunks[3]);
            }
        }

        // Instructions
        let instructions_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CELESTIAL_BLUE))
            .title(Span::styled(
                "✧ Instructions",
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
        let status = Paragraph::new(self.status.clone())
            .style(Style::default().fg(AURORA_GREEN))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(status, chunks[6]);

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

fn main() -> Result<(), io::Error> {
    let initial_message = String::from("feat: Implement cosmic TUI for Git-Iris");
    let custom_instructions = String::from("Channel the cosmic energy to craft a commit message that aligns with the celestial Conventional Commits specification. Focus on the main changes and their impact on the cosmic codebase.");
    let mut app = TuiCommit::new(initial_message, custom_instructions);
    app.run()
}
