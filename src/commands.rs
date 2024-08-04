use crate::config::Config;
use crate::git::get_git_info;
use crate::interactive::InteractiveCommit;
use crate::llm::get_refined_message;
use crate::log_debug;
use crate::messages;
use crate::ui;
use anyhow::{anyhow, Result};
use clap::{crate_name, crate_version};
use std::collections::HashMap;
use std::sync::Arc;

/// Handle the 'gen' command
pub async fn handle_gen_command(
    verbose: bool,
    use_gitmoji: bool,
    provider: Option<String>,
    auto_commit: bool,
    instructions: Option<String>,
) -> Result<()> {
    log_debug!(
        "Starting 'gen' command with verbose: {}, use_gitmoji: {}, provider: {:?}, auto_commit: {}, instructions: {:?}",
        verbose,
        use_gitmoji,
        provider,
        auto_commit,
        instructions
    );

    let config = Arc::new(Config::load()?);

    // Check environment prerequisites
    if let Err(e) = Config::check_environment() {
        ui::print_error(&format!("Error: {}", e));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info("2. You are running this command from within a Git repository.");
        ui::print_info("3. You have set up your configuration using 'git-iris config'.");
        return Ok(());
    }

    let provider = Arc::new(provider.unwrap_or_else(|| config.default_provider.clone()));
    let provider_config = config
        .get_provider_config(&provider)
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider))?;

    if provider_config.api_key.is_empty() {
        ui::print_error(&format!("API key for provider '{}' is not set. Please run 'git-iris config --provider {} --api-key YOUR_API_KEY' to set it.", provider, provider));
        return Ok(());
    }

    let current_dir = Arc::new(std::env::current_dir()?);

    let message = messages::get_random_message();
    let spinner = ui::create_spinner(&message);

    let git_info = get_git_info(
        current_dir.as_path(),
        &config,
        Some(&|progress_msg: &str| {
            spinner.set_message(progress_msg.to_string());
        }),
    )?;

    if git_info.staged_files.is_empty() {
        spinner.finish_and_clear();
        ui::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let use_gitmoji = use_gitmoji && config.use_gitmoji;

    // Get instructions
    let instructions = instructions.unwrap_or_else(|| config.instructions.clone());

    // Update spinner message before generating the initial message
    spinner.set_message(messages::get_random_message());
    
    // Generate the initial message
    let initial_message = get_refined_message(
        &git_info,
        &config,
        &provider,
        use_gitmoji,
        verbose,
        &instructions,
    )
    .await?;

    spinner.finish_and_clear();

    // Initialize interactive commit process with program name and version
    let mut interactive_commit = InteractiveCommit::new(
        initial_message,
        instructions,
        crate_name!().to_string(),
        crate_version!().to_string(),
    );

    // Run the interactive commit process
    let commit_performed = interactive_commit
        .run(move |instructions| {
            let config = Arc::clone(&config);
            let provider = Arc::clone(&provider);
            let current_dir = Arc::clone(&current_dir);
            let use_gitmoji = use_gitmoji;
            let verbose = verbose;
            let instructions = instructions.to_string();
            async move {
                let git_info = get_git_info(current_dir.as_path(), &config, None)?;
                get_refined_message(
                    &git_info,
                    &config,
                    &provider,
                    use_gitmoji,
                    verbose,
                    &instructions,
                )
                .await
            }
        })
        .await?;

    if commit_performed {
        log_debug!("Commit successfully created and applied.");
    } else {
        log_debug!("Commit process cancelled.");
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
    instructions: Option<String>,
    token_limit: Option<usize>,
) -> Result<()> {
    log_debug!("Starting 'config' command with provider: {:?}, api_key: {:?}, model: {:?}, param: {:?}, gitmoji: {:?}, instructions: {:?}, token_limit: {:?}", provider, api_key, model, param, gitmoji, instructions, token_limit);

    let mut config = Config::load()?;

    let provider = provider.map(|p| p.to_string());
    let additional_params = param.map(|p| parse_additional_params(&p));

    config.update(
        provider,
        api_key,
        model,
        additional_params,
        gitmoji,
        instructions,
        token_limit,
    );
    config.save()?;
    ui::print_success("Configuration updated successfully.");
    ui::print_info(&format!(
        "Current configuration:\nDefault Provider: {}\nUse Gitmoji: {}\nInstructions: {}",
        config.default_provider,
        config.use_gitmoji,
        if config.instructions.is_empty() {
            "None".to_string()
        } else {
            config.instructions.replace('\n', ", ")
        }
    ));
    for (provider, provider_config) in &config.providers {
        ui::print_info(&format!(
            "\nProvider: {}\nAPI Key: {}\nModel: {}\nToken Limit: {}\nAdditional Parameters: {:?}",
            provider,
            if provider_config.api_key.is_empty() {
                "Not set"
            } else {
                "Set"
            },
            provider_config.model,
            provider_config
                .token_limit
                .map_or("Default".to_string(), |limit| limit.to_string()),
            provider_config.additional_params
        ));
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
