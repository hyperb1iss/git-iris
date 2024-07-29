use crate::config::Config;
use crate::git::get_git_info;
use crate::interactive::InteractiveCommit;
use crate::llm::get_refined_message;
use crate::log_debug;
use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use clap::{crate_name, crate_version};

/// Handle the 'gen' command
pub async fn handle_gen_command(
    verbose: bool,
    gitmoji: Option<bool>,
    provider: Option<String>,
    _auto_commit: bool,
) -> Result<()> {
    log_debug!(
        "Starting 'gen' command with verbose: {}, gitmoji: {:?}, provider: {:?}",
        verbose,
        gitmoji,
        provider
    );

    let config = Arc::new(Config::load()?);

    // Check environment prerequisites
    if let Err(e) = Config::check_environment() {
        println!("Error: {}", e);
        println!("\nPlease ensure the following:");
        println!("1. Git is installed and accessible from the command line.");
        println!("2. You are running this command from within a Git repository.");
        println!("3. You have set up your configuration using 'git-iris config'.");
        return Ok(());
    }

    let provider = Arc::new(provider.unwrap_or_else(|| config.default_provider.clone()));
    let provider_config = config
        .get_provider_config(&provider)
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider))?;

    if provider_config.api_key.is_empty() {
        println!("API key for provider '{}' is not set. Please run 'git-iris config --provider {} --api-key YOUR_API_KEY' to set it.", provider, provider);
        return Ok(());
    }

    let current_dir = Arc::new(std::env::current_dir()?);
    let git_info = get_git_info(current_dir.as_path())?;

    if git_info.staged_files.is_empty() {
        println!(
            "No staged changes. Please stage your changes before generating a commit message."
        );
        println!("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let use_gitmoji = gitmoji.unwrap_or(config.use_gitmoji);

    // Display a spinner while generating the message
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner} Generating initial commit message...")?,
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Get custom instructions
    let custom_instructions = config.custom_instructions.clone();

    // Generate the initial message
    let initial_message = get_refined_message(
        &git_info,
        &config,
        &provider,
        use_gitmoji,
        verbose,
        None,
        &custom_instructions,
    )
    .await?;

    spinner.finish_and_clear();

    // Initialize interactive commit process with program name and version
    let mut interactive_commit = InteractiveCommit::new(
        initial_message,
        custom_instructions,
        crate_name!().to_string(),
        crate_version!().to_string(),
    );

    // Run the interactive commit process
    let commit_performed = interactive_commit
        .run(move |existing_message, custom_instructions| {
            let config = Arc::clone(&config);
            let provider = Arc::clone(&provider);
            let current_dir = Arc::clone(&current_dir);
            let use_gitmoji = use_gitmoji;
            let verbose = verbose;
            let custom_instructions = custom_instructions.to_string();
            async move {
                let git_info = get_git_info(current_dir.as_path())?;
                get_refined_message(
                    &git_info,
                    &config,
                    &provider,
                    use_gitmoji,
                    verbose,
                    existing_message.as_deref(),
                    &custom_instructions,
                )
                .await
            }
        })
        .await?;

    if commit_performed {
        println!("Commit successfully created and applied.");
    } else {
        println!("Commit process cancelled.");
    }

    Ok(())
}

/// Handle the 'config' command
pub fn handle_config_command(
    provider: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    param: Option<Vec<String>>,
    gitmoji: Option<bool>,
    custom_instructions: Option<String>,
) -> Result<()> {
    log_debug!("Starting 'config' command with provider: {:?}, api_key: {:?}, model: {:?}, param: {:?}, gitmoji: {:?}, custom_instructions: {:?}", provider, api_key, model, param, gitmoji, custom_instructions);

    let mut config = Config::load()?;

    let provider = provider.map(|p| p.to_string());
    let additional_params = param.map(|p| parse_additional_params(&p));

    config.update(
        provider,
        api_key,
        model,
        additional_params,
        gitmoji,
        custom_instructions,
    );
    config.save()?;
    println!("Configuration updated successfully.");
    println!(
        "Current configuration:\nDefault Provider: {}\nUse Gitmoji: {}\nCustom Instructions: {}",
        config.default_provider,
        config.use_gitmoji,
        if config.custom_instructions.is_empty() {
            "None".to_string()
        } else {
            config.custom_instructions.replace('\n', ", ")
        }
    );
    for (provider, provider_config) in &config.providers {
        println!(
            "\nProvider: {}\nAPI Key: {}\nModel: {}\nAdditional Parameters: {:?}",
            provider,
            if provider_config.api_key.is_empty() {
                "Not set"
            } else {
                "Set"
            },
            provider_config.model,
            provider_config.additional_params
        );
    }

    Ok(())
}

/// Parse additional parameters from the command line
fn parse_additional_params(params: &[String]) -> HashMap<String, String> {
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
