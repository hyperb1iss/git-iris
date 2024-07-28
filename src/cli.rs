use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about = "AI-assisted Git commit message generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Automatically commit with the generated message")]
    pub auto_commit: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Gen {
        #[arg(short, long, help = "Enable verbose mode")]
        verbose: bool,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}