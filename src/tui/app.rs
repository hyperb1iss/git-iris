use crate::commit::IrisCommitService;
use crate::context::GeneratedMessage;
use crate::log_debug;
use anyhow::{Error, Result};
use crossterm::event::KeyEventKind;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use super::input_handler::{handle_input, InputResult};
use super::spinner::SpinnerState;
use super::state::{EmojiMode, Mode, TuiState};
use super::ui::draw_ui;

pub struct TuiCommit {
    pub state: TuiState,
    service: Arc<IrisCommitService>,
}

impl TuiCommit {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        preset: String,
        user_name: String,
        user_email: String,
        service: Arc<IrisCommitService>,
        use_gitmoji: bool,
    ) -> Self {
        let state = TuiState::new(
            initial_messages,
            custom_instructions,
            preset,
            user_name,
            user_email,
            use_gitmoji,
        );

        TuiCommit { state, service }
    }

    pub async fn run(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        selected_preset: String,
        user_name: String,
        user_email: String,
        service: Arc<IrisCommitService>,
        use_gitmoji: bool,
    ) -> Result<()> {
        let mut app = TuiCommit::new(
            initial_messages,
            custom_instructions,
            selected_preset,
            user_name,
            user_email,
            service,
            use_gitmoji,
        );
        app.run_app().await.map_err(Error::from)
    }

    pub async fn run_app(&mut self) -> io::Result<()> {
        // Setup
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run main loop
        let result = self.main_loop(&mut terminal).await;

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // Handle result and display appropriate message
        match result {
            Ok(exit_status) => match exit_status {
                ExitStatus::Committed(_message) => {
                    println!("Commit successful!");
                }
                ExitStatus::Cancelled => {
                    println!("Commit operation cancelled. Your changes remain staged.");
                }
                ExitStatus::Error(error_message) => {
                    eprintln!("An error occurred: {}", error_message);
                }
            },
            Err(e) => {
                eprintln!("An unexpected error occurred: {}", e);
                return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
            }
        }

        Ok(())
    }

    async fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<ExitStatus> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<GeneratedMessage, anyhow::Error>>(1);
        let mut task_spawned = false;

        loop {
            // Redraw only if dirty
            if self.state.dirty {
                terminal.draw(|f| draw_ui(f, &mut self.state))?;
                self.state.dirty = false; // Reset dirty flag after redraw
            }

            // Spawn the task only once when entering the Generating mode
            if self.state.mode == Mode::Generating && !task_spawned {
                let service = self.service.clone();
                let preset = self.state.selected_preset.clone();
                let instructions = self.state.custom_instructions.clone();
                let tx = tx.clone();

                tokio::spawn(async move {
                    log_debug!("Generating message...");
                    let result = service.generate_message(&preset, &instructions).await;
                    let _ = tx.send(result).await;
                });

                task_spawned = true; // Ensure we only spawn the task once
            }

            // Check if a message has been received from the generation task
            match rx.try_recv() {
                Ok(result) => match result {
                    Ok(new_message) => {
                        let current_emoji_mode = self.state.emoji_mode.clone();
                        self.state.messages.push(new_message);
                        self.state.current_index = self.state.messages.len() - 1;

                        // Apply the current emoji mode to the new message
                        if let Some(message) = self.state.messages.last_mut() {
                            match &current_emoji_mode {
                                EmojiMode::None => message.emoji = None,
                                EmojiMode::Auto => {} // Keep the LLM-generated emoji
                                EmojiMode::Custom(emoji) => message.emoji = Some(emoji.clone()),
                            }
                        }
                        self.state.emoji_mode = current_emoji_mode;

                        self.state.update_message_textarea();
                        self.state.mode = Mode::Normal; // Exit Generating mode
                        self.state.spinner = None; // Stop the spinner
                        self.state
                            .set_status(String::from("New message generated successfully!"));
                        task_spawned = false; // Reset for future regenerations
                    }
                    Err(e) => {
                        self.state.mode = Mode::Normal; // Exit Generating mode
                        self.state.spinner = None; // Stop the spinner
                        self.state
                            .set_status(format!("Failed to generate new message: {}. Press 'r' to retry or 'Esc' to exit.", e));
                        task_spawned = false; // Reset for future regenerations
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No message available yet, continue the loop
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Handle the case where the sender has disconnected
                    break;
                }
            }

            // Poll for input events
            if event::poll(Duration::from_millis(20))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match handle_input(self, key) {
                            InputResult::Exit => return Ok(ExitStatus::Cancelled),
                            InputResult::Commit(message) => match self.perform_commit(&message) {
                                Ok(status) => return Ok(status),
                                Err(e) => {
                                    self.state.set_status(format!("Commit failed: {}", e));
                                    self.state.dirty = true;
                                }
                            },
                            InputResult::Continue => self.state.dirty = true,
                        }
                    }
                }
            }

            // Update the spinner state and redraw if in generating mode
            if self.state.mode == Mode::Generating {
                if self.state.last_spinner_update.elapsed() >= Duration::from_millis(100) {
                    if let Some(spinner) = &mut self.state.spinner {
                        spinner.tick();
                        self.state.dirty = true; // Mark dirty to trigger redraw
                    }
                    self.state.last_spinner_update = std::time::Instant::now(); // Reset the update time
                }
            }
        }

        Ok(ExitStatus::Cancelled)
    }

    pub fn handle_regenerate(&mut self) {
        self.state.mode = Mode::Generating;
        self.state.spinner = Some(SpinnerState::new());
    }

    pub fn perform_commit(&self, message: &str) -> Result<ExitStatus, Error> {
        match self.service.perform_commit(message) {
            Ok(()) => Ok(ExitStatus::Committed(message.to_string())),
            Err(e) => Ok(ExitStatus::Error(e.to_string())),
        }
    }
}

pub async fn run_tui_commit(
    initial_messages: Vec<GeneratedMessage>,
    custom_instructions: String,
    selected_preset: String,
    user_name: String,
    user_email: String,
    service: Arc<IrisCommitService>,
    use_gitmoji: bool,
) -> Result<()> {
    TuiCommit::run(
        initial_messages,
        custom_instructions,
        selected_preset,
        user_name,
        user_email,
        service,
        use_gitmoji,
    )
    .await
}

pub enum ExitStatus {
    Committed(String),
    Cancelled,
    Error(String),
}
