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
    inpaint_context: Vec<String>,
}

impl InteractiveCommit {
    pub fn new(initial_message: String) -> Self {
        InteractiveCommit {
            messages: vec![initial_message],
            current_index: 0,
            inpaint_context: Vec::new(),
        }
    }

    pub async fn run<F, Fut>(&mut self, generate_message: F) -> Result<bool>
    where
        F: Fn(&[String]) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut term = Term::stdout();
        loop {
            term.clear_screen()?;
            self.display_current_message(&mut term)?;

            match term.read_key()? {
                Key::ArrowLeft => self.navigate_left(),
                Key::ArrowRight => self.navigate_right(&generate_message).await?,
                Key::Char('e') | Key::Char('E') => {
                    if let Some(edited_message) = self.edit_message()? {
                        self.messages[self.current_index] = edited_message;
                    }
                }
                Key::Char('i') | Key::Char('I') => {
                    self.add_inpaint_context(&mut term)?;
                    self.regenerate_message(&generate_message).await?;
                }
                Key::Enter => {
                    return self.perform_commit();
                }
                Key::Escape => {
                    return Ok(false);
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
        if !self.inpaint_context.is_empty() {
            writeln!(term, "{}", inpaint_style.apply_to("Inpaint Context:"))?;
            for context in &self.inpaint_context {
                writeln!(term, "- {}", inpaint_style.apply_to(context))?;
            }
            writeln!(term)?;
        }
        writeln!(
            term,
            "{}",
            prompt_style.apply_to("← → Navigate | e Edit | i Inpaint | Enter Commit | Esc Cancel")
        )?;

        Ok(())
    }

    fn navigate_left(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    async fn navigate_right<F, Fut>(&mut self, generate_message: &F) -> Result<()>
    where
        F: Fn(&[String]) -> Fut,
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

            let new_message = generate_message(&self.inpaint_context).await?;
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
        F: Fn(&[String]) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} Regenerating commit message...")?,
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        let new_message = generate_message(&self.inpaint_context).await?;
        self.messages[self.current_index] = new_message;

        spinner.finish_and_clear();
        Ok(())
    }

    fn add_inpaint_context(&mut self, term: &mut Term) -> Result<()> {
        let prompt_style = Style::new().yellow();
        writeln!(
            term,
            "{}",
            prompt_style.apply_to("Enter additional context (empty line to finish):")
        )?;

        loop {
            let input = term.read_line()?;
            let input = input.trim();

            if input.is_empty() {
                break;
            }

            self.inpaint_context.push(input.to_string());
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

        let success_style = Style::new().green().bold();
        let error_style = Style::new().red().bold();

        match result {
            Ok(_) => {
                println!("{}", success_style.apply_to("✅ Commit successful!"));
                Ok(true)
            }
            Err(e) => {
                println!("{} {}", error_style.apply_to("❌ Commit failed:"), e);
                Ok(false)
            }
        }
    }
}
