use anyhow::Result;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
use std::io::Write;
use std::process::Command;

pub fn interactive_commit(message: &str) -> Result<bool> {
    let term = Term::stdout();
    term.clear_screen()?;

    println!("📝 Proposed Commit Message:\n");
    println!("{}\n", message);

    let options = vec!["Commit", "Generate New Message", "Edit Message", "Cancel"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(&options)
        .default(0)
        .interact_on(&Term::stderr())?;

    match selection {
        0 => perform_commit(message),
        1 => Ok(false), // Generate new message
        2 => edit_message(message),
        3 => {
            println!("Commit cancelled.");
            Ok(true)
        }
        _ => unreachable!(),
    }
}

fn perform_commit(message: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(&["commit", "-m", message])
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

fn edit_message(message: &str) -> Result<bool> {
    let mut file = tempfile::NamedTempFile::new()?;
    write!(file, "{}", message)?;

    let path = file.into_temp_path();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let status = Command::new(editor).arg(&path).status()?;

    if status.success() {
        let edited_message = std::fs::read_to_string(&path)?;
        perform_commit(&edited_message)
    } else {
        println!("Message editing cancelled.");
        Ok(false)
    }
}
