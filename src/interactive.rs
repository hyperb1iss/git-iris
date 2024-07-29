use crate::git;
use anyhow::Result;
use console::{Key, Style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::process::Command;
use std::time::Duration;

pub struct InteractiveCommit {
    messages: Vec<String>,
    current_index: usize,
    generating: bool,
    custom_instructions: String,
}

impl InteractiveCommit {
    pub fn new(initial_message: String, custom_instructions: String) -> Self {
        InteractiveCommit {
            messages: vec![initial_message],
            current_index: 0,
            generating: false,
            custom_instructions,
        }
    }

    pub async fn run<F, Fut>(&mut self, generate_message: F) -> Result<bool>
    where
        F: Fn(Option<String>, &str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut term = Term::stdout();
        loop {
            term.clear_screen()?;
            self.display_current_message(&mut term)?;

            match term.read_key()? {
                Key::ArrowLeft => {
                    if !self.generating {
                        self.navigate_left();
                    }
                }
                Key::ArrowRight => {
                    if !self.generating {
                        self.generating = true;
                        self.navigate_right(&generate_message).await?;
                        self.generating = false;
                    }
                }
                Key::Char('e') | Key::Char('E') => {
                    if !self.generating {
                        if let Some(edited_message) = self.edit_message()? {
                            self.messages[self.current_index] = edited_message;
                        }
                    }
                }
                Key::Char('i') | Key::Char('I') => {
                    if !self.generating {
                        self.edit_custom_instructions(&generate_message).await?;
                    }
                }
                Key::Enter => {
                    if !self.generating {
                        return self.perform_commit();
                    }
                }
                Key::Escape => {
                    if !self.generating {
                        return Ok(false);
                    }
                }
                _ => {}
            }
        }
    }

    fn display_current_message(&self, term: &mut Term) -> Result<()> {
        let title_style = Style::new().cyan().bold();
        let prompt_style = Style::new().yellow();
        let value_style = Style::new().green();
        let inpaint_style = Style::new().magenta();

        writeln!(
            term,
            "{} ({}/{})",
            title_style.apply_to("📝 Commit Message"),
            self.current_index + 1,
            self.messages.len()
        )?;
        writeln!(term)?;
        writeln!(
            term,
            "{}",
            value_style.apply_to(&self.messages[self.current_index])
        )?;
        writeln!(term)?;

        if !self.custom_instructions.trim().is_empty() {
            writeln!(term, "{}", inpaint_style.apply_to("Custom Instructions:"))?;
            writeln!(term, "{}", value_style.apply_to(&self.custom_instructions))?;
            writeln!(term)?;
        }

        if self.generating {
            writeln!(
                term,
                "{}",
                prompt_style.apply_to("Generating message... Please wait.")
            )?;
        } else {
            writeln!(
                term,
                "{}",
                prompt_style.apply_to(
                    "← → Navigate | e Edit | i Edit Instructions | Enter Commit | Esc Cancel"
                )
            )?;
        }

        Ok(())
    }

    fn navigate_left(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    async fn navigate_right<F, Fut>(&mut self, generate_message: &F) -> Result<()>
    where
        F: Fn(Option<String>, &str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        if self.current_index == self.messages.len() - 1 {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                    .template("{spinner} Generating commit message...")?,
            );
            spinner.enable_steady_tick(Duration::from_millis(100));

            let new_message =
                generate_message(None, &self.custom_instructions).await?;
            spinner.finish_and_clear();

            self.messages.push(new_message);
        }
        self.current_index += 1;
        Ok(())
    }

    fn edit_message(&self) -> Result<Option<String>> {
        let mut file = tempfile::NamedTempFile::new()?;
        write!(file, "{}", self.messages[self.current_index])?;

        let path = file.into_temp_path();
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let status = Command::new(editor).arg(&path).status()?;

        if status.success() {
            let edited_message = std::fs::read_to_string(&path)?;
            Ok(Some(edited_message))
        } else {
            println!("Message editing cancelled.");
            Ok(None)
        }
    }

    async fn regenerate_message<F, Fut>(&mut self, generate_message: &F) -> Result<()>
    where
        F: Fn(Option<String>, &str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} Regenerating commit message...")?,
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        let existing_message = self.messages[self.current_index].clone();
        let new_message = generate_message(
            Some(existing_message),
            &self.custom_instructions,
        )
        .await?;
        self.messages[self.current_index] = new_message;

        spinner.finish_and_clear();
        Ok(())
    }

    async fn edit_custom_instructions<F, Fut>(&mut self, generate_message: &F) -> Result<()>
    where
        F: Fn(Option<String>, &str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut file = tempfile::NamedTempFile::new()?;
        write!(file, "{}", self.custom_instructions)?;

        let path = file.into_temp_path();
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let status = Command::new(editor).arg(&path).status()?;

        if status.success() {
            let edited_instructions = std::fs::read_to_string(&path)?;
            self.custom_instructions = edited_instructions;
            self.regenerate_message(generate_message).await?;
        } else {
            println!("Editing custom instructions cancelled.");
        }

        Ok(())
    }

    fn perform_commit(&self) -> Result<bool> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} Committing changes...")?,
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        let commit_message = &self.messages[self.current_index];
        let repo_path = std::env::current_dir()?;
        let result = git::commit(&repo_path, commit_message);

        spinner.finish_and_clear();

        match result {
            Ok(_) => {
                println!("✅ Commit successful!");
                Ok(true)
            }
            Err(e) => {
                println!("❌ Commit failed: {}", e);
                Ok(false)
            }
        }
    }
}
