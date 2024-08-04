use crate::commands;
use crate::llm::get_available_provider_names;
use crate::log_debug;
use crate::ui;
use clap::builder::{styling::AnsiColor, Styles};
use clap::{crate_version, Parser, Subcommand};

/// CLI structure defining the available commands and global arguments
#[derive(Parser)]
#[command(
    author,
    version = crate_version!(),
    about = "AI-assisted Git commit message generator",
    long_about = None,
    disable_version_flag = true,
    after_help = get_dynamic_help(),
    styles = get_styles(),
)]
pub struct Cli {
    /// Subcommands available for the CLI
    #[command(subcommand)]
    pub command: Option<Commands>,

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
        long_about = "Generate a commit message using AI based on the current Git context.",
        after_help = get_dynamic_help()
    )]
    Gen {
        /// Automatically commit with the generated message
        #[arg(short, long, help = "Automatically commit with the generated message")]
        auto_commit: bool,

        /// Custom instructions for this commit
        #[arg(short, long, help = "Custom instructions for this commit")]
        instructions: Option<String>,

        /// Override default LLM provider
        #[arg(long, help = "Override default LLM provider", value_parser = available_providers_parser)]
        provider: Option<String>,

        /// Disable Gitmoji for this commit
        #[arg(long, help = "Disable Gitmoji for this commit")]
        no_gitmoji: bool,
    },
    /// Configure the AI-assisted Git commit message generator
    #[command(about = "Configure the AI-assisted Git commit message generator")]
    Config {
        /// Set default LLM provider
        #[arg(long, help = "Set default LLM provider", value_parser = available_providers_parser)]
        provider: Option<String>,

        /// Set API key for the specified provider
        #[arg(long, help = "Set API key for the specified provider")]
        api_key: Option<String>,

        /// Set model for the specified provider
        #[arg(long, help = "Set model for the specified provider")]
        model: Option<String>,

        /// Set token limit for the specified provider
        #[arg(long, help = "Set token limit for the specified provider")]
        token_limit: Option<usize>,

        /// Set additional parameters for the specified provider
        #[arg(
            long,
            help = "Set additional parameters for the specified provider (key=value)"
        )]
        param: Option<Vec<String>>,

        /// Set Gitmoji usage preference
        #[arg(long, help = "Enable or disable Gitmoji")]
        gitmoji: Option<bool>,

        /// Set instructions for the commit message generation
        #[arg(
            short,
            long,
            help = "Set instructions for the commit message generation"
        )]
        instructions: Option<String>,
    },
}

/// Define custom styles for Clap
fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Magenta.on_default().bold())
        .usage(AnsiColor::Cyan.on_default().bold())
        .literal(AnsiColor::Green.on_default().bold())
        .placeholder(AnsiColor::Yellow.on_default())
        .valid(AnsiColor::Blue.on_default().bold())
        .invalid(AnsiColor::Red.on_default().bold())
        .error(AnsiColor::Red.on_default().bold())
}

/// Parse the command-line arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Generate dynamic help including available LLM providers
fn get_dynamic_help() -> String {
    let providers = get_available_provider_names().join(", ");
    format!("Available providers: {}", providers)
}

/// Validate provider input against available providers
fn available_providers_parser(s: &str) -> Result<String, String> {
    let available_providers = get_available_provider_names();
    if available_providers.contains(&s.to_lowercase()) {
        Ok(s.to_lowercase())
    } else {
        Err(format!(
            "Invalid provider. Available providers are: {}",
            available_providers.join(", ")
        ))
    }
}

/// Main function to parse arguments and handle the command
pub async fn main() -> anyhow::Result<()> {
    let cli = parse_args();

    if cli.version {
        ui::print_version(crate_version!());
        return Ok(());
    }

    if cli.log {
        crate::logger::enable_logging();
    } else {
        crate::logger::disable_logging();
    }

    match cli.command {
        Some(command) => handle_command(command, cli.log).await?,
        None => {
            // If no subcommand is provided, print the help
            let _ = Cli::parse_from(&["git-iris", "--help"]);
        }
    }

    Ok(())
}

/// Handle the command based on parsed arguments
pub async fn handle_command(command: Commands, log: bool) -> anyhow::Result<()> {
    match command {
        Commands::Gen {
            auto_commit,
            instructions,
            provider,
            no_gitmoji,
        } => {
            log_debug!(
                "Handling 'gen' command with auto_commit: {}, instructions: {:?}, provider: {:?}, no_gitmoji: {}",
                auto_commit,
                instructions,
                provider,
                no_gitmoji
            );

            ui::print_version(crate_version!());
            println!();

            commands::handle_gen_command(log, !no_gitmoji, provider, auto_commit, instructions)
                .await?;
        }
        Commands::Config {
            provider,
            api_key,
            model,
            param,
            gitmoji,
            instructions,
            token_limit,
        } => {
            log_debug!("Handling 'config' command with provider: {:?}, api_key: {:?}, model: {:?}, param: {:?}, gitmoji: {:?}, instructions: {:?}, token_limit: {:?}", provider, api_key, model, param, gitmoji, instructions, token_limit);
            ui::print_info("Updating configuration...");
            commands::handle_config_command(
                provider,
                api_key,
                model,
                param,
                gitmoji,
                instructions,
                token_limit,
            )?;
        }
    }

    Ok(())
}
