use anyhow::Result;
use console::{Key, Term};
use std::future::Future;
use std::io::Write;
use std::process::Command;

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
        Fut: Future<Output = Result<String>>,
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
                Key::Char('c') | Key::Char('C') => {
                    return self.perform_commit();
                }
                Key::Escape => {
                    println!("Commit cancelled.");
                    return Ok(false);
                }
                _ => {}
            }
        }
    }

    fn display_current_message(&self, term: &mut Term) -> Result<()> {
        writeln!(
            term,
            "📝 Commit Message ({}/{}):\n",
            self.current_index + 1,
            self.messages.len()
        )?;
        writeln!(term, "{}\n", self.messages[self.current_index])?;
        writeln!(term, "Navigation: ←/→ | e: Edit | c: Commit | Esc: Cancel")?;
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
        Fut: Future<Output = Result<String>>,
    {
        if self.current_index == self.messages.len() - 1 {
            let new_message = generate_message().await?;
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
        let output = Command::new("git")
            .args(&["commit", "-m", &self.messages[self.current_index]])
            .output()?;

        if output.status.success() {
            println!("✅ Commit successful!");
            Ok(true)
        } else {
            println!(
                "❌ Commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(false)
        }
    }
}
