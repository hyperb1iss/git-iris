use super::changelog::ChangelogGenerator;
use super::releasenotes::ReleaseNotesGenerator;
use crate::common::{CommonParams, DetailLevel};
use crate::config::Config;
use crate::git::GitRepo;
use crate::ui;
use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::str::FromStr;
use std::sync::Arc;

/// Handles the changelog generation command.
///
/// This function orchestrates the process of generating a changelog based on the provided
/// parameters. It sets up the necessary environment, creates a `GitRepo` instance,
/// and delegates the actual generation to the `ChangelogGenerator`.
///
/// # Arguments
///
/// * `common` - Common parameters for the command, including configuration overrides.
/// * `from` - The starting point (commit or tag) for the changelog.
/// * `to` - The ending point for the changelog. Defaults to "HEAD" if not provided.
///
/// # Returns
///
/// Returns a Result indicating success or containing an error if the operation failed.
pub async fn handle_changelog_command(
    common: CommonParams,
    from: String,
    to: Option<String>,
) -> Result<()> {
    // Load and apply configuration
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Create a spinner to indicate progress
    let spinner = ui::create_spinner("Generating changelog...");

    // Ensure we're in a git repository
    if let Err(e) = config.check_environment() {
        ui::print_error(&format!("Error: {e}"));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info("2. You are running this command from within a Git repository.");
        ui::print_info("3. You have set up your configuration using 'git-iris config'.");
        return Err(e);
    }

    // Get the current directory and create a GitRepo instance
    let repo_path = env::current_dir()?;
    let git_repo = Arc::new(GitRepo::new(&repo_path).context("Failed to create GitRepo")?);

    // Set the default 'to' reference if not provided
    let to = to.unwrap_or_else(|| "HEAD".to_string());

    // Parse the detail level for the changelog
    let detail_level = DetailLevel::from_str(&common.detail_level)?;

    // Generate the changelog
    let changelog =
        ChangelogGenerator::generate(git_repo, &from, &to, &config, detail_level).await?;

    // Clear the spinner and display the result
    spinner.finish_and_clear();

    println!("{}", "━".repeat(50).bright_purple());
    println!("{}", &changelog);
    println!("{}", "━".repeat(50).bright_purple());

    Ok(())
}

/// Handles the release notes generation command.
///
/// This function orchestrates the process of generating release notes based on the provided
/// parameters. It sets up the necessary environment, creates a `GitRepo` instance,
/// and delegates the actual generation to the `ReleaseNotesGenerator`.
///
/// # Arguments
///
/// * `common` - Common parameters for the command, including configuration overrides.
/// * `from` - The starting point (commit or tag) for the release notes.
/// * `to` - The ending point for the release notes. Defaults to "HEAD" if not provided.
///
/// # Returns
///
/// Returns a Result indicating success or containing an error if the operation failed.
pub async fn handle_release_notes_command(
    common: CommonParams,
    from: String,
    to: Option<String>,
) -> Result<()> {
    // Load and apply configuration
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Create a spinner to indicate progress
    let spinner = ui::create_spinner("Generating release notes...");

    // Check environment prerequisites
    if let Err(e) = config.check_environment() {
        ui::print_error(&format!("Error: {e}"));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info("2. You are running this command from within a Git repository.");
        ui::print_info("3. You have set up your configuration using 'git-iris config'.");
        return Err(e);
    }

    // Get the current directory and create a GitRepo instance
    let repo_path = env::current_dir()?;
    let git_repo = Arc::new(GitRepo::new(&repo_path).context("Failed to create GitRepo")?);

    // Set the default 'to' reference if not provided
    let to = to.unwrap_or_else(|| "HEAD".to_string());

    // Parse the detail level for the release notes
    let detail_level = DetailLevel::from_str(&common.detail_level)?;

    // Generate the release notes
    let release_notes =
        ReleaseNotesGenerator::generate(git_repo, &from, &to, &config, detail_level).await?;

    // Clear the spinner and display the result
    spinner.finish_and_clear();

    println!("{}", "━".repeat(50).bright_purple());
    println!("{}", &release_notes);
    println!("{}", "━".repeat(50).bright_purple());

    Ok(())
}
