use crate::git;
use crate::log_debug;
use crate::messages;
use anyhow::Result;
use colored::*;
use console::{Key, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::max;
use std::io::Write;
use std::process::Command;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

pub struct InteractiveCommit {
    messages: Vec<String>,
    current_index: usize,
    generating: bool,
    custom_instructions: String,
    program_name: String,
    program_version: String,
}

impl InteractiveCommit {
    pub fn new(
        initial_message: String,
        custom_instructions: String,
        program_name: String,
        program_version: String,
    ) -> Self {
        InteractiveCommit {
            messages: vec![initial_message],
            current_index: 0,
            generating: false,
            custom_instructions,
            program_name,
            program_version,
        }
    }

    pub async fn run<F, Fut>(&mut self, generate_message: F) -> Result<bool>
    where
        F: Fn(&str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut term = Term::stdout();
        loop {
            term.clear_screen()?;
            self.display_current_message(&mut term)?;

            match term.read_key()? {
                Key::ArrowLeft => {
                    if !self.generating && self.current_index > 0 {
                        self.current_index -= 1;
                    }
                }
                Key::ArrowRight => {
                    if !self.generating && self.current_index < self.messages.len() - 1 {
                        self.current_index += 1;
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
                Key::Char('r') | Key::Char('R') => {
                    if !self.generating {
                        self.generating = true;
                        self.regenerate_message(&generate_message).await?;
                        self.current_index = self.messages.len() - 1;
                        self.generating = false;
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
        let term_width = (term.size().1 - 1) as usize;

        self.display_header(term, term_width)?;

        let current_message_number = self.current_index + 1;
        let total_messages = self.messages.len();
        let title = format!(
            "Commit Message ({}/{})",
            current_message_number, total_messages
        );

        self.display_title(term, &title, '✦', term_width)?;

        self.display_message_box(term, &self.messages[self.current_index], term_width)?;

        writeln!(term)?;

        if !self.custom_instructions.trim().is_empty() {
            self.display_title(term, "Custom Instructions", '✧', term_width)?;
            self.display_message_box(term, &self.custom_instructions, term_width)?;
            writeln!(term)?;
        }

        if self.generating {
            writeln!(
                term,
                "{}",
                "🔮 Divining the perfect commit message... Please wait.".bright_purple()
            )?;
        } else {
            self.display_navigation_hints(term)?;
        }

        Ok(())
    }

    fn display_header(&self, term: &mut Term, term_width: usize) -> Result<()> {
        let logo = self.create_gradient_logo();
        let logo_width = console::strip_ansi_codes(&logo).width();
        let logo_padding = if term_width > logo_width {
            (term_width - logo_width) / 2
        } else {
            0
        };
        writeln!(term, "{}{}", " ".repeat(logo_padding), logo)?;

        let star_pattern = "・。.・゜✭・.・✫・゜・";
        let colored_star_pattern: String = star_pattern
            .chars()
            .map(|c| c.to_string().truecolor(147, 112, 219).bold().to_string())
            .collect();
        let pattern_width = console::strip_ansi_codes(&star_pattern).width();
        let pattern_padding = if logo_width > pattern_width {
            logo_padding + (logo_width - pattern_width) / 2
        } else {
            max(logo_padding, (term_width.saturating_sub(pattern_width)) / 2)
        };
        writeln!(
            term,
            "{}{}",
            " ".repeat(pattern_padding),
            colored_star_pattern
        )?;
        writeln!(term)?;
        Ok(())
    }

    fn display_title(
        &self,
        term: &mut Term,
        title: &str,
        symbol: char,
        term_width: usize,
    ) -> Result<()> {
        let title = format!(" {} ", title);
        let gradient_title = self.create_secondary_gradient_text(&title);
        let symbol_str = symbol
            .to_string()
            .truecolor(147, 112, 219)
            .bold()
            .to_string();
        let title_width = console::strip_ansi_codes(&gradient_title).width();

        let padding = term_width.saturating_sub(title_width + 4); // 4 for the symbols on each side
        let left_padding = padding / 2;
        let right_padding = padding - left_padding;
        writeln!(
            term,
            "{}{}{}{}",
            symbol_str.repeat(2),
            gradient_title,
            " ".repeat(left_padding + right_padding),
            symbol_str.repeat(2)
        )?;
        Ok(())
    }

    fn display_message_box(&self, term: &mut Term, message: &str, term_width: usize) -> Result<()> {
        let max_width = term_width.saturating_sub(4).max(1); // Ensure at least 1 character width
        let wrapped_message = textwrap::fill(message, max_width);
        let lines: Vec<&str> = wrapped_message.lines().collect();

        let border_color = Color::TrueColor {
            r: 147,
            g: 112,
            b: 219,
        };
        let content_color = Color::TrueColor {
            r: 173,
            g: 216,
            b: 230,
        }; // Light blue

        let top_border = "┏".color(border_color).to_string()
            + &"━".color(border_color).to_string().repeat(max_width + 2)
            + &"┓".color(border_color).to_string();
        writeln!(term, "{}", top_border)?;

        for line in lines.iter() {
            let line_width = line.width();
            let padding = if max_width > line_width {
                max_width - line_width
            } else {
                0
            };
            let formatted_line = format!(
                "{} {}{} {}",
                "┃".color(border_color),
                line.color(content_color).bold(),
                " ".repeat(padding),
                "┃".color(border_color)
            );
            writeln!(term, "{}", formatted_line)?;
        }

        let bottom_border = "┗".color(border_color).to_string()
            + &"━".color(border_color).to_string().repeat(max_width + 2)
            + &"┛".color(border_color).to_string();
        writeln!(term, "{}", bottom_border)?;

        Ok(())
    }

    fn display_navigation_hints(&self, term: &mut Term) -> Result<()> {
        let hints = vec![
            ("←→", "Navigate", (147, 112, 219), "🔮"),
            ("e", "Edit", (0, 255, 255), "✏️"),
            ("i", "Instructions", (138, 43, 226), "📜"),
            ("r", "Regenerate", (0, 191, 255), "✨"),
            ("Enter", "Commit", (123, 104, 238), "💫"),
            ("Esc", "Cancel", (255, 20, 147), "🌠"),
        ];

        let mut hint_line = String::new();
        for (i, (key, action, color, emoji)) in hints.iter().enumerate() {
            if i > 0 {
                hint_line.push_str("  ");
            }
            hint_line.push_str(&format!(
                "{} {} {}",
                emoji,
                key.bold(),
                format!("{} {}", ":", action)
                    .truecolor(color.0, color.1, color.2)
                    .bold()
            ));
        }

        let term_width = term.size().1 as usize;
        let padded_hint_line = format!("{:^width$}", hint_line, width = term_width);

        term.write_line(&padded_hint_line)?;

        Ok(())
    }

    fn create_gradient_logo(&self) -> String {
        let logo = format!("🔮 {} v{} 🔮", self.program_name, self.program_version);
        self.create_gradient_text(&logo)
    }

    fn create_gradient_text(&self, text: &str) -> String {
        let gradient = vec![
            (129, 0, 255), // Deep purple
            (134, 51, 255),
            (139, 102, 255),
            (144, 153, 255),
            (149, 204, 255), // Light cyan
        ];

        self.apply_gradient(text, &gradient)
    }

    fn create_secondary_gradient_text(&self, text: &str) -> String {
        let gradient = vec![
            (75, 0, 130),   // Indigo
            (106, 90, 205), // Slate blue
            (138, 43, 226), // Blue violet
            (148, 0, 211),  // Dark violet
            (153, 50, 204), // Dark orchid
        ];

        self.apply_gradient(text, &gradient)
    }

    fn apply_gradient(&self, text: &str, gradient: &[(u8, u8, u8)]) -> String {
        let chars: Vec<char> = text.chars().collect();
        let step = (gradient.len() - 1) as f32 / (chars.len() - 1) as f32;

        chars
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let index = (i as f32 * step) as usize;
                let (r, g, b) = gradient[index];
                format!("{}", c.to_string().truecolor(r, g, b))
            })
            .collect()
    }

    async fn regenerate_message<F, Fut>(&mut self, generate_message: &F) -> Result<()>
    where
        F: Fn(&str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let spinner = ProgressBar::new_spinner();
        let random_message = messages::get_random_message();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("✦✧✶✷✸✹✺✻✼✽")
                .template(&format!("{{spinner}} {}", random_message))?,
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        let new_message = generate_message(&self.custom_instructions).await?;
        self.messages.push(new_message);
        self.current_index = self.messages.len() - 1;

        spinner.finish_and_clear();
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
            log_debug!("✏️ Message edited: {}", edited_message);
            Ok(Some(edited_message))
        } else {
            println!("🌠 Message editing cancelled.");
            Ok(None)
        }
    }

    async fn edit_custom_instructions<F, Fut>(&mut self, generate_message: &F) -> Result<()>
    where
        F: Fn(&str) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut file = tempfile::NamedTempFile::new()?;
        write!(file, "{}", self.custom_instructions)?;

        let path = file.into_temp_path();
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let status = Command::new(editor).arg(&path).status()?;

        if status.success() {
            let edited_instructions = std::fs::read_to_string(&path)?;
            log_debug!("📜 Custom instructions edited: {}", edited_instructions);
            self.custom_instructions = edited_instructions;
            self.regenerate_message(generate_message).await?;
        } else {
            println!("🌠 Editing custom instructions cancelled.");
        }

        Ok(())
    }

    fn perform_commit(&self) -> Result<bool> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("✦✧✶✷✸✹✺✻✼✽")
                .template("{spinner} 💫 Committing changes...")?,
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        let commit_message = &self.messages[self.current_index];
        let repo_path = std::env::current_dir()?;
        let result = git::commit(&repo_path, commit_message);

        spinner.finish_and_clear();

        match result {
            Ok(_) => {
                println!(
                    "{}",
                    "✨ Commit successful! The stars have aligned."
                        .truecolor(147, 112, 219)
                        .bold()
                );
                log_debug!("✨ Commit successful with message: {}", commit_message);
                Ok(true)
            }
            Err(e) => {
                println!(
                    "{} {}",
                    "🌠 Commit failed:".truecolor(255, 20, 147).bold(),
                    e
                );
                log_debug!("🌠 Commit failed: {}", e);
                Ok(false)
            }
        }
    }
}
