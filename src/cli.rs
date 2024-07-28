use clap::{Parser, Subcommand};
use colored::*;

#[derive(Parser)]
#[command(author, version, about = "AI-assisted Git commit message generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        short,
        long,
        global = true,
        help = "Automatically commit with the generated message"
    )]
    pub auto_commit: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Gen {
        #[arg(short, long, help = "Enable verbose mode")]
        verbose: bool,

        #[arg(short, long, help = "Override use_gitmoji setting")]
        gitmoji: Option<bool>,
    },
    Config {
        #[arg(short, long, help = "Set OpenAI API key")]
        api_key: Option<String>,

        #[arg(short, long, help = "Set use_gitmoji preference")]
        gitmoji: Option<bool>,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

pub fn print_success(message: &str) {
    println!("{}", message.green());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}

pub fn print_info(message: &str) {
    println!("{}", message.blue());
}

pub fn print_warning(message: &str) {
    println!("{}", message.yellow());
}
