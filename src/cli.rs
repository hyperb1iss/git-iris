use clap::{Parser, Subcommand, ValueEnum}; // Added ValueEnum
use colored::*;
use std::fmt; // Added import for fmt

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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)] // Added Debug
pub enum LLMImplementation {
    OpenAI,
    Claude,
}

// Implementing Display trait for LLMImplementation
impl fmt::Display for LLMImplementation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self) // This uses the Debug representation, you can customize it if needed
    }
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
        #[arg(short, long, help = "Set API key")]
        api_key: Option<String>,

        #[arg(long, help = "Set LLM provider", value_enum)]
        llm_provider: Option<LLMImplementation>,

        #[arg(short, long, help = "Set use_gitmoji preference")]
        gitmoji: Option<bool>,

        #[arg(
            short,
            long,
            help = "Set custom instructions (separate multiple instructions with newlines)"
        )]
        custom_instructions: Option<String>,
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
