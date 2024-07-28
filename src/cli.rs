use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use std::collections::HashMap;
use std::fmt;

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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum LLMImplementation {
    OpenAI,
    Claude,
}

impl fmt::Display for LLMImplementation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Subcommand)]
pub enum Commands {
    Gen {
        #[arg(short, long, help = "Enable verbose mode")]
        verbose: bool,

        #[arg(short, long, help = "Override use_gitmoji setting")]
        gitmoji: Option<bool>,

        #[arg(long, help = "Override default LLM provider")]
        provider: Option<LLMImplementation>,
    },
    Config {
        #[arg(long, help = "Set default LLM provider", value_enum)]
        provider: Option<LLMImplementation>,

        #[arg(long, help = "Set API key for the specified provider")]
        api_key: Option<String>,

        #[arg(long, help = "Set model for the specified provider")]
        model: Option<String>,

        #[arg(
            long,
            help = "Set additional parameters for the specified provider (key=value)"
        )]
        param: Option<Vec<String>>,

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

pub fn parse_additional_params(params: &[String]) -> HashMap<String, String> {
    params
        .iter()
        .filter_map(|param| {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}
