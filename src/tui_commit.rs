use std::io;
use std::time::Duration;
use ratatui::{
    backend::CrosstermBackend, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap}, Frame, Terminal
};
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui_textarea::TextArea;

mod gitmoji;
use gitmoji::get_gitmoji_list;
mod instruction_presets;
use instruction_presets::{get_instruction_preset_library, list_presets_formatted};

#[derive(PartialEq)]
enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    SelectingEmoji,
    SelectingPreset,
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
                    parts[0].to_string(),  // key
                    parts[1].to_string(),  // emoji
                    parts[2].to_string(),  // name
                    parts[3].to_string(),  // description
                )
            })
            .collect();

        let mut preset_list_state = ListState::default();
        preset_list_state.select(Some(0));

        let mut preset_list_state = ListState::default();
        preset_list_state.select(Some(0));

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
                        },
                        Mode::EditingMessage => self.handle_editing_message(key),
                        Mode::EditingInstructions => self.handle_editing_instructions(key),
                        Mode::SelectingEmoji => self.handle_selecting_emoji(key),
                        Mode::SelectingPreset => self.handle_selecting_preset(key),
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
                self.status = String::from("Selecting emoji. Use arrow keys and Enter to select, Esc to cancel.");
            }
            KeyCode::Char('p') => {
                self.mode = Mode::SelectingPreset;
                self.status = String::from("Selecting preset. Use arrow keys and Enter to select, Esc to cancel.");
            }
            _ => {}
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
                    let new_selected = if selected > 0 { selected - 1 } else { self.emoji_list.len() - 1 };
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
                    self.status = format!("Preset selected: {}", self.get_selected_preset_name_with_emoji());
                }
            }
            KeyCode::Up => {
                let selected = self.preset_list_state.selected().unwrap_or(0);
                let new_selected = if selected > 0 { selected - 1 } else { self.preset_list.len() - 1 };
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
                let new_selected = if selected + 10 < self.preset_list.len() { selected + 10 } else { self.preset_list.len() - 1 };
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
                    Constraint::Length(3), // Navigation bar
                    Constraint::Length(3), // User info
                    Constraint::Min(5),    // Commit message
                    Constraint::Length(5), // Instructions
                    Constraint::Length(3), // Emoji and Preset
                    Constraint::Length(1), // Status
                ]
                .as_ref(),
            )
            .split(f.size());

        // Title
        let title = Paragraph::new("🔮 Git-Iris v0.1.0 - Cosmic Commit 🔮")
            .style(Style::default().fg(Color::Magenta))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(title, chunks[0]);

        // Navigation bar
        let nav_text = "🌠 ←→: Navigate  🌟 E: Edit Message  🌙 I: Edit Instructions  🎨 G: Select Emoji  ✨ R: Regenerate  💫 ⏎: Commit";
        let nav_bar = Paragraph::new(nav_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(nav_bar, chunks[1]);

        // User info
        let user_info = Paragraph::new(Line::from(vec![
            Span::styled("👤 ", Style::default().fg(Color::Cyan)),
            Span::styled(&self.user_name, Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled("✉️ ", Style::default().fg(Color::Cyan)),
            Span::styled(&self.user_email, Style::default().fg(Color::Green)),
        ]))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(user_info, chunks[2]);

        // Commit message
        let message_title = format!("✦ Commit Message ({}/{})", self.current_index + 1, self.messages.len());
        let message_block = Block::default().borders(Borders::ALL).title(message_title);
        match self.mode {
            Mode::EditingMessage => {
                self.message_textarea.set_block(message_block);
                f.render_widget(&self.message_textarea, chunks[3]);
            }
            _ => {
                let message = Paragraph::new(self.messages[self.current_index].clone())
                    .block(message_block)
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: true });
                f.render_widget(message, chunks[3]);
            }
        }

        // Instructions
        let instructions_block = Block::default().borders(Borders::ALL).title("✧ Instructions");
        match self.mode {
            Mode::EditingInstructions => {
                self.instructions_textarea.set_block(instructions_block);
                f.render_widget(&self.instructions_textarea, chunks[4]);
            }
            _ => {
                let instructions = Paragraph::new(self.custom_instructions.clone())
                    .block(instructions_block)
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: true });
                f.render_widget(instructions, chunks[4]);
            }
        }

        // Emoji and Preset
        let binding = self.get_selected_preset_name_with_emoji();
        let emoji_preset = Paragraph::new(Line::from(vec![
            Span::styled("Emoji: ", Style::default().fg(Color::Cyan)),
            Span::styled(&self.selected_emoji, Style::default().fg(Color::Yellow)),
            Span::raw("  |  "),
            Span::styled("Preset: ", Style::default().fg(Color::Cyan)),
            Span::styled(&binding, Style::default().fg(Color::Yellow)),
        ]))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(emoji_preset, chunks[5]);

        // Status
        let status = Paragraph::new(self.status.clone())
            .style(Style::default().fg(Color::Cyan))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(status, chunks[6]);

        if self.mode == Mode::SelectingEmoji {
            self.render_emoji_popup(f);
        } else if self.mode == Mode::SelectingPreset {
            self.render_preset_popup(f);
        }
    }

    fn render_emoji_popup(&mut self, f: &mut Frame) {
        let popup_block = Block::default()
            .title("✨ Select Emoji")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));
    
        let area = f.size();
        let popup_area = Rect::new(
            area.x + 10,
            area.y + 5,
            area.width.saturating_sub(20).min(60),
            area.height.saturating_sub(10).min(20),
        );
    
        let items: Vec<ListItem> = self.emoji_list
            .iter()
            .map(|(emoji, description)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", emoji),
                        Style::default().fg(Color::Yellow)
                    ),
                    Span::raw(description),
                ]))
            })
            .collect();
    
        let list = List::new(items)
            .block(popup_block)
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD)
            );
    
        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut self.emoji_list_state);
    }

    fn get_selected_preset_name_with_emoji(&self) -> String {
        self.preset_list.iter()
            .find(|(key, _, _, _)| key == &self.selected_preset)
            .map(|(_, emoji, name, _)| format!("{} {}", emoji, name))
            .unwrap_or_else(|| "None".to_string())
    }

    fn render_preset_popup(&mut self, f: &mut Frame) {
        let popup_block = Block::default()
            .title("🌟 Select Preset")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));
    
        let area = f.size();
        let popup_area = Rect::new(
            area.x + 5,
            area.y + 5,
            area.width.saturating_sub(10).min(70), // Wider popup
            area.height.saturating_sub(10).min(20),
        );
    
        let items: Vec<ListItem> = self.preset_list
            .iter()
            .map(|(_, emoji, name, description)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} {} ", emoji, name),
                        Style::default().fg(Color::Yellow)
                    ),
                    Span::raw(description),
                ]))
            })
            .collect();
    
        let list = List::new(items)
            .block(popup_block)
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD)
            );
        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut self.preset_list_state);
    }
}

fn main() -> Result<(), io::Error> {
    let initial_message = String::from("feat: Implement cosmic TUI for Git-Iris");
    let custom_instructions = String::from("Channel the cosmic energy to craft a commit message that aligns with the celestial Conventional Commits specification. Focus on the main changes and their impact on the cosmic codebase.");
    let mut app = TuiCommit::new(initial_message, custom_instructions);
    app.run()
}