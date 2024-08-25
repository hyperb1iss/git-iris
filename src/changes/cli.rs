use super::changelog::ChangelogGenerator;
use super::releasenotes::ReleaseNotesGenerator;
use crate::common::{CommonParams, DetailLevel};
use crate::config::Config;
use crate::ui;
use anyhow::Result;
use colored::Colorize;
use std::env;
use std::str::FromStr;

pub async fn handle_changelog_command(
    common: CommonParams,
    from: String,
    to: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;
    let spinner = ui::create_spinner("Generating changelog...");

    let repo_path = env::current_dir()?;
    let to = to.unwrap_or_else(|| "HEAD".to_string());

    // Parse detail level
    let detail_level = DetailLevel::from_str(&common.detail_level)?;

    let changelog =
        ChangelogGenerator::generate(&repo_path, &from, &to, &config, detail_level).await?;

    spinner.finish_and_clear();

    println!("{}", "━".repeat(50).bright_purple());
    println!("{}", &changelog);
    println!("{}", "━".repeat(50).bright_purple());

    Ok(())
}

pub async fn handle_release_notes_command(
    common: CommonParams,
    from: String,
    to: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;
    let spinner = ui::create_spinner("Generating release notes...");

    let repo_path = env::current_dir()?;
    let to = to.unwrap_or_else(|| "HEAD".to_string());

    // Parse detail level
    let detail_level = DetailLevel::from_str(&common.detail_level)?;

    let release_notes =
        ReleaseNotesGenerator::generate(&repo_path, &from, &to, &config, detail_level).await?;

    spinner.finish_and_clear();

    println!("{}", "━".repeat(50).bright_purple());
    println!("{}", &release_notes);
    println!("{}", "━".repeat(50).bright_purple());

    Ok(())
}
