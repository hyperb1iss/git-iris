use colored::Colorize;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};
use ratatui::style::Color;
use std::fmt::Write;
use std::time::Duration;

pub const STARLIGHT: Color = Color::Rgb(255, 255, 240);
pub const NEBULA_PURPLE: Color = Color::Rgb(167, 132, 239);
pub const CELESTIAL_BLUE: Color = Color::Rgb(75, 115, 235);
pub const SOLAR_YELLOW: Color = Color::Rgb(255, 225, 100);
pub const AURORA_GREEN: Color = Color::Rgb(140, 255, 170);
pub const PLASMA_CYAN: Color = Color::Rgb(20, 255, 255);
pub const METEOR_RED: Color = Color::Rgb(255, 89, 70);
pub const GALAXY_PINK: Color = Color::Rgb(255, 162, 213);
pub const COMET_ORANGE: Color = Color::Rgb(255, 165, 0);
pub const BLACK_HOLE: Color = Color::Rgb(0, 0, 0);

pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("✦✧✶✷✸✹✺✻✼✽")
            .template("{spinner} {msg}")
            .expect("Could not set spinner style"),
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
    let chars_len = chars.len();
    let gradient_len = gradient.len();

    let mut result = String::new();

    if chars_len == 0 || gradient_len == 0 {
        return result;
    }

    chars.iter().enumerate().fold(&mut result, |acc, (i, &c)| {
        let index = if chars_len == 1 {
            0
        } else {
            i * (gradient_len - 1) / (chars_len - 1)
        };
        let (r, g, b) = gradient[index];
        write!(acc, "{}", c.to_string().truecolor(r, g, b)).unwrap();
        acc
    });

    result
}

pub fn write_gradient_text(
    term: &Term,
    text: &str,
    gradient: &[(u8, u8, u8)],
) -> std::io::Result<()> {
    let gradient_text = apply_gradient(text, gradient);
    term.write_line(&gradient_text)
}

pub fn write_colored_text(term: &Term, text: &str, color: (u8, u8, u8)) -> std::io::Result<()> {
    let colored_text = text.truecolor(color.0, color.1, color.2);
    term.write_line(&colored_text)
}

pub fn write_bold_text(term: &Term, text: &str) -> std::io::Result<()> {
    term.write_line(&text.bold())
}
