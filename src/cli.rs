use crate::commands;
use crate::log_debug;
use clap::{crate_version, Parser, Subcommand};
use colored::*;

/// CLI structure defining the available commands and global arguments
#[derive(Parser)]
#[command(
    author,
    version = crate_version!(),
    about = "AI-assisted Git commit message generator",
    long_about = None,
    disable_version_flag = true
)]
pub struct Cli {
    /// Subcommands available for the CLI
    #[command(subcommand)]
    pub command: Commands,

    /// Automatically commit with the generated message
    #[arg(
        short,
        long,
        global = true,
        help = "Automatically commit with the generated message"
    )]
    pub auto_commit: bool,

    /// Log debug messages to a file
    #[arg(
        short = 'l',
        long = "log",
        global = true,
        help = "Log debug messages to a file"
    )]
    pub log: bool,

    /// Display the version
    #[arg(
        short = 'v',
        long = "version",
        global = true,
        help = "Display the version"
    )]
    pub version: bool,
}

/// Enumeration of available subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Generate a commit message using AI
    #[command(
        about = "Generate a commit message using AI",
        after_help = "Default LLM provider: openai\nAvailable providers: claude, openai"
    )]
    Gen {
        #[arg(short = 'g', long, help = "Override use_gitmoji setting")]
        gitmoji: Option<bool>,

        #[arg(long, help = "Override default LLM provider")]
        provider: Option<String>,
    },
    /// Configure the AI-assisted Git commit message generator
    #[command(about = "Configure the AI-assisted Git commit message generator")]
    Config {
        #[arg(long, help = "Set default LLM provider")]
        provider: Option<String>,

        #[arg(long, help = "Set API key for the specified provider")]
        api_key: Option<String>,

        #[arg(long, help = "Set model for the specified provider")]
        model: Option<String>,

        #[arg(long, help = "Set custom token limit for the specified provider")]
        token_limit: Option<usize>,

        #[arg(
            long,
            help = "Set additional parameters for the specified provider (key=value)"
        )]
        param: Option<Vec<String>>,

        #[arg(short = 'g', long, help = "Set use_gitmoji preference")]
        gitmoji: Option<bool>,

        #[arg(
            short = 'c',
            long,
            help = "Set custom instructions for the commit message generation"
        )]
        custom_instructions: Option<String>,
    },
}

/// Parse the command-line arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Print an informational message with blue color
pub fn print_info(message: &str) {
    println!("{}", message.blue());
}

/// Print a warning message with yellow color
pub fn print_warning(message: &str) {
    println!("{}", message.yellow());
}

/// List available LLM providers
pub fn list_providers() -> Vec<String> {
    // Query the provider registry to get the list of available providers
    crate::provider_registry::ProviderRegistry::default().list_providers()
}

/// Print dynamic help including available LLM providers
pub fn print_dynamic_help() {
    let providers = list_providers();
    let provider_list = providers.join(", ");
    println!("Available providers: {}", provider_list);
}

/// Main function to parse arguments and handle the command
pub async fn main() -> anyhow::Result<()> {
    let cli = parse_args();

    if cli.version {
        println!("Version: {}", crate_version!());
        return Ok(());
    }

    if cli.log {
        crate::logger::enable_logging();
    } else {
        crate::logger::disable_logging();
    }

    handle_command(cli).await
}

/// Handle the command based on parsed arguments
pub async fn handle_command(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Gen {
            gitmoji,
            provider,
        } => {
            log_debug!(
                "Handling 'gen' command with gitmoji: {:?}, provider: {:?}",
                gitmoji,
                provider
            );
            commands::handle_gen_command(cli.log, gitmoji, provider, cli.auto_commit).await?;
        }
        Commands::Config {
            provider,
            api_key,
            model,
            param,
            gitmoji,
            custom_instructions,
            token_limit,
        } => {
            log_debug!("Handling 'config' command with provider: {:?}, api_key: {:?}, model: {:?}, param: {:?}, gitmoji: {:?}, custom_instructions: {:?}", provider, api_key, model, param, gitmoji, custom_instructions);
            commands::handle_config_command(
                provider,
                api_key,
                model,
                param,
                gitmoji,
                custom_instructions,
                token_limit,
            )?;
        }
    }

    Ok(())
}
