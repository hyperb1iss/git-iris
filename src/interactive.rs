use anyhow::Result;
use console::{Key, Style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::process::Command;
use std::time::Duration;

pub struct InteractiveCommit {
    messages: Vec<String>,
    current_index: usize,
}

impl InteractiveCommit {
    pub fn new(initial_message: String) -> Self {
        InteractiveCommit {
            messages: vec![initial_message],
            current_index: 0,
        }
    }

    pub async fn run<F, Fut>(&mut self, generate_message: F) -> Result<bool>
    where
        F: Fn() -> Fut,
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
        writeln!(
            term,
            "{}",
            prompt_style.apply_to(format!("← → Navigate | e Edit | Enter Commit | Esc Cancel"))
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
        F: Fn() -> Fut,
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

            let new_message = generate_message().await?;
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

    fn perform_commit(&self) -> Result<bool> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} Committing changes...")?,
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        let output = Command::new("git")
            .args(&["commit", "-m", &self.messages[self.current_index]])
            .output()?;

        spinner.finish_and_clear();

        let success_style = Style::new().green().bold();
        let error_style = Style::new().red().bold();

        if output.status.success() {
            println!("{}", success_style.apply_to("✅ Commit successful!"));
            Ok(true)
        } else {
            println!(
                "{} {}",
                error_style.apply_to("❌ Commit failed:"),
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(false)
        }
    }
}
