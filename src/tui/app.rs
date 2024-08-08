use crate::context::{format_commit_message, GeneratedMessage};
use anyhow::{Error, Result};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use super::input_handler::{handle_input, InputResult};
use super::spinner::SpinnerState;
use super::state::{Mode, TuiState};
use super::ui::draw_ui;

pub struct TuiCommit {
    pub state: TuiState,
    generate_message: Arc<dyn Fn(&str, &str) + Send + Sync>,
    pub perform_commit: Arc<dyn Fn(&str) -> Result<(), Error> + Send + Sync>,
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
        let state = TuiState::new(
            initial_messages,
            custom_instructions,
            preset,
            user_name,
            user_email,
        );

        TuiCommit {
            state,
            generate_message,
            perform_commit,
            message_receiver,
        }
    }

    pub fn run(
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
        app.run_app().map_err(Error::from)
    }

    pub fn run_app(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.main_loop(&mut terminal);

        // restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| draw_ui(f, &mut self.state))?;

            if self.state.mode == Mode::Generating {
                match self.message_receiver.try_recv() {
                    Ok(result) => match result {
                        Ok(new_message) => {
                            self.state.messages.push(new_message);
                            self.state.current_index = self.state.messages.len() - 1;
                            self.state.update_message_textarea();
                            self.state.mode = Mode::Normal;
                            self.state
                                .set_status(String::from("New message generated successfully!"));
                        }
                        Err(e) => {
                            self.state.mode = Mode::Normal;
                            self.state
                                .set_status(format!("Failed to generate new message: {}", e));
                        }
                    },
                    Err(mpsc::error::TryRecvError::Empty) => {}
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Message channel disconnected",
                        ));
                    }
                }
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match handle_input(self, key) {
                        InputResult::Exit => break,
                        InputResult::Continue => {}
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_regenerate(&mut self) {
        self.state.mode = Mode::Generating;
        self.state.spinner = Some(SpinnerState::new());
        (self.generate_message)(&self.state.selected_preset, &self.state.custom_instructions);
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
    TuiCommit::run(
        initial_messages,
        custom_instructions,
        selected_preset,
        user_name,
        user_email,
        generate_message,
        perform_commit,
        message_receiver,
    )
}
