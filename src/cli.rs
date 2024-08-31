use crate::changes;
use crate::commands;
use crate::commit;
use crate::common::CommonParams;
use crate::llm::get_available_provider_names;
use crate::log_debug;
use crate::ui;
use clap::builder::{styling::AnsiColor, Styles};
use clap::{crate_version, Parser, Subcommand};

const LOG_FILE: &str = "git-iris-debug.log";

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
        #[command(flatten)]
        common: CommonParams,

        /// Automatically commit with the generated message
        #[arg(short, long, help = "Automatically commit with the generated message")]
        auto_commit: bool,

        /// Disable Gitmoji for this commit
        #[arg(long, help = "Disable Gitmoji for this commit")]
        no_gitmoji: bool,

        /// Print the generated message to stdout and exit
        #[arg(short, long, help = "Print the generated message to stdout and exit")]
        print: bool,

        /// Skip the verification step (pre/post commit hooks)
        #[arg(long, help = "Skip verification steps (pre/post commit hooks)")]
        no_verify: bool,
    },
    /// Configure the AI-assisted Git commit message generator
    #[command(about = "Configure the AI-assisted Git commit message generator")]
    Config {
        #[command(flatten)]
        common: CommonParams,

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
    },

    /// List available instruction presets
    #[command(about = "List available instruction presets")]
    ListPresets,

    /// Generate a changelog
    #[command(
        about = "Generate a changelog",
        long_about = "Generate a changelog between two specified Git references."
    )]
    Changelog {
        #[command(flatten)]
        common: CommonParams,

        /// Starting Git reference (commit hash, tag, or branch name)
        #[arg(long, required = true)]
        from: String,

        /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
        #[arg(long)]
        to: Option<String>,
    },
    /// Generate release notes
    #[command(
        about = "Generate release notes",
        long_about = "Generate comprehensive release notes between two specified Git references."
    )]
    ReleaseNotes {
        #[command(flatten)]
        common: CommonParams,

        /// Starting Git reference (commit hash, tag, or branch name)
        #[arg(long, required = true)]
        from: String,

        /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
        #[arg(long)]
        to: Option<String>,
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
    format!("Available providers: {providers}")
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
        crate::logger::set_log_file(LOG_FILE)?;
    } else {
        crate::logger::disable_logging();
    }

    if let Some(command) = cli.command {
        handle_command(command).await
    } else {
        // If no subcommand is provided, print the help
        let _ = Cli::parse_from(["git-iris", "--help"]);
        Ok(())
    }
}

/// Handle the command based on parsed arguments
pub async fn handle_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Gen {
            common,
            auto_commit,
            no_gitmoji,
            print,
            no_verify,
        } => {
            log_debug!(
                "Handling 'gen' command with common: {:?}, auto_commit: {}, no_gitmoji: {}, print: {}, no_verify: {}",
                common, auto_commit, no_gitmoji, print, no_verify
            );

            ui::print_version(crate_version!());
            println!();

            commit::handle_gen_command(common, auto_commit, !no_gitmoji, print, !no_verify).await?;
        }
        Commands::Config {
            common,
            api_key,
            model,
            token_limit,
            param,
        } => {
            log_debug!(
                "Handling 'config' command with common: {:?}, api_key: {:?}, model: {:?}, token_limit: {:?}, param: {:?}",
                common, api_key, model, token_limit, param
            );
            commands::handle_config_command(common, api_key, model, token_limit, param)?;
        }
        Commands::ListPresets => {
            log_debug!("Handling 'list_presets' command");
            commands::handle_list_presets_command()?;
        }
        Commands::Changelog { common, from, to } => {
            log_debug!(
                "Handling 'changelog' command with common: {:?}, from: {}, to: {:?}",
                common,
                from,
                to
            );
            changes::handle_changelog_command(common, from, to).await?;
        }
        Commands::ReleaseNotes { common, from, to } => {
            log_debug!(
                "Handling 'release-notes' command with common: {:?}, from: {}, to: {:?}",
                common,
                from,
                to
            );
            changes::handle_release_notes_command(common, from, to).await?;
        }
    }

    Ok(())
}
