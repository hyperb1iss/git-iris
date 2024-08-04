use colored::*;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("✦✧✶✷✸✹✺✻✼✽")
            .template("{spinner} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn print_info(message: &str) {
    println!("{}", message.cyan().bold());
}

pub fn print_warning(message: &str) {
    println!("{}", message.yellow().bold());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.red().bold());
}

pub fn print_success(message: &str) {
    println!("{}", message.green().bold());
}

pub fn print_version(version: &str) {
    println!(
        "{} {} {}",
        "🔮 Git-Iris".magenta().bold(),
        "version".cyan(),
        version.green()
    );
}

pub fn create_gradient_text(text: &str) -> String {
    let gradient = vec![
        (129, 0, 255), // Deep purple
        (134, 51, 255),
        (139, 102, 255),
        (144, 153, 255),
        (149, 204, 255), // Light cyan
    ];

    apply_gradient(text, &gradient)
}

pub fn create_secondary_gradient_text(text: &str) -> String {
    let gradient = vec![
        (75, 0, 130),   // Indigo
        (106, 90, 205), // Slate blue
        (138, 43, 226), // Blue violet
        (148, 0, 211),  // Dark violet
        (153, 50, 204), // Dark orchid
    ];

    apply_gradient(text, &gradient)
}

fn apply_gradient(text: &str, gradient: &[(u8, u8, u8)]) -> String {
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

pub fn write_gradient_text(
    term: &mut Term,
    text: &str,
    gradient: &[(u8, u8, u8)],
) -> std::io::Result<()> {
    let gradient_text = apply_gradient(text, gradient);
    term.write_line(&gradient_text)
}

pub fn write_colored_text(term: &mut Term, text: &str, color: (u8, u8, u8)) -> std::io::Result<()> {
    let colored_text = text.truecolor(color.0, color.1, color.2);
    term.write_line(&colored_text)
}

pub fn write_bold_text(term: &mut Term, text: &str) -> std::io::Result<()> {
    term.write_line(&text.bold())
}
