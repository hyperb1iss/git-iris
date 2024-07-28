use crate::commands;
use clap::{Parser, Subcommand};
use colored::*;

#[derive(Parser)]
#[command(author, version, about = "AI-assisted Git commit message generator", long_about = None)]
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
    #[command(
        about = "Generate a commit message using AI",
        after_help = "Default LLM provider: openai\nAvailable providers: claude, openai"
    )]
    Gen {
        #[arg(short, long, help = "Enable verbose mode")]
        verbose: bool,

        #[arg(short, long, help = "Override use_gitmoji setting")]
        gitmoji: Option<bool>,

        #[arg(long, help = "Override default LLM provider")]
        provider: Option<String>,
    },
    #[command(about = "Configure the AI-assisted Git commit message generator")]
    Config {
        #[arg(long, help = "Set default LLM provider")]
        provider: Option<String>,

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
    let cli = Cli::parse();
    if let Commands::Gen { .. } = cli.command {
        if std::env::args().any(|arg| arg == "--help") {
            print_dynamic_help();
        }
    }
    cli
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

pub fn list_providers() -> Vec<String> {
    // Query the provider registry to get the list of available providers
    crate::provider_registry::ProviderRegistry::default().list_providers()
}

pub fn print_dynamic_help() {
    let providers = list_providers();
    let provider_list = providers.join(", ");
    println!("Available providers: {}", provider_list);
}

pub async fn main() -> anyhow::Result<()> {
    let cli = parse_args();
    handle_command(cli).await
}

pub async fn handle_command(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Gen {
            verbose,
            gitmoji,
            provider,
        } => {
            commands::handle_gen_command(verbose, gitmoji, provider, cli.auto_commit).await?;
        }
        Commands::Config {
            provider,
            api_key,
            model,
            param,
            gitmoji,
            custom_instructions,
        } => {
            commands::handle_config_command(
                provider,
                api_key,
                model,
                param,
                gitmoji,
                custom_instructions,
            )?;
        }
    }

    Ok(())
}
