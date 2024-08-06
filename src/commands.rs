use crate::changelog::{ChangelogGenerator, ReleaseNotesGenerator};
use crate::config::Config;
use crate::git::get_git_info;
use crate::instruction_presets::get_instruction_preset_library;
use crate::interactive::InteractiveCommit;
use crate::llm::get_refined_message;
use crate::llm_providers::{get_available_providers, get_provider_metadata, LLMProviderType};
use crate::log_debug;
use crate::messages;
use crate::prompt;
use crate::token_optimizer::TokenOptimizer;
use crate::ui;
use anyhow::{anyhow, Result};
use clap::{crate_name, crate_version};
use colored::*;
use std::collections::HashMap;
use std::env;
use std::sync::Arc; // Add this line
use unicode_width::UnicodeWidthStr;

/// Handle the 'gen' command
pub async fn handle_gen_command(
    use_gitmoji: bool,
    provider: Option<String>,
    auto_commit: bool,
    custom_instructions: Option<String>,
    preset: Option<String>,
    print: bool,
) -> Result<()> {
    log_debug!(
        "Starting 'gen' command with use_gitmoji: {}, provider: {:?}, auto_commit: {}, custom_instructions: {:?}, preset: {:?}, print: {}",
        use_gitmoji,
        provider,
        auto_commit,
        custom_instructions,
        preset,
        print
    );

    let config = Config::load()?;

    // Check environment prerequisites
    if let Err(e) = Config::check_environment() {
        ui::print_error(&format!("Error: {}", e));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info("2. You are running this command from within a Git repository.");
        ui::print_info("3. You have set up your configuration using 'git-iris config'.");
        return Ok(());
    }

    let provider_type = if let Some(p) = provider {
        LLMProviderType::from_str(&p)?
    } else {
        LLMProviderType::from_str(&config.default_provider)?
    };

    let provider_metadata = get_provider_metadata(&provider_type);

    if provider_metadata.requires_api_key {
        let provider_config = config
            .get_provider_config(&provider_type.to_string())
            .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider_type))?;

        if provider_config.api_key.is_empty() {
            ui::print_error(&format!("API key for provider '{}' is not set. Please run 'git-iris config --provider {} --api-key YOUR_API_KEY' to set it.", provider_type, provider_type));
            return Ok(());
        }
    }

    let current_dir = Arc::new(std::env::current_dir()?);

    let message = messages::get_random_message();
    let spinner = ui::create_spinner(&message);

    let mut git_info = get_git_info(current_dir.as_path(), &config)?;

    if git_info.staged_files.is_empty() {
        spinner.finish_and_clear();
        ui::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let use_gitmoji = use_gitmoji && config.use_gitmoji;

    // Get instructions from preset and/or custom instructions
    let preset_library = get_instruction_preset_library();
    let preset_key = preset.unwrap_or(config.instruction_preset.clone());
    let preset_instructions = preset_library
        .get_preset(&preset_key)
        .map(|p| p.instructions.clone())
        .unwrap_or_default();

    let custom_instructions = custom_instructions.unwrap_or_else(|| config.instructions.clone());

    let combined_instructions = format!(
        "{}\n\n{}",
        preset_instructions.trim(),
        custom_instructions.trim()
    )
    .trim()
    .to_string();

    // Update spinner message before generating the initial message
    spinner.set_message(messages::get_random_message());

    // Token optimization
    let token_limit = provider_metadata.default_token_limit;
    let optimizer = TokenOptimizer::new(token_limit);
    optimizer.optimize_context(&mut git_info);

    let system_prompt = prompt::create_system_prompt(use_gitmoji, &combined_instructions);
    let user_prompt = prompt::create_user_prompt(&git_info)?;

    // Generate the initial message
    let initial_message = get_refined_message(
        &config,
        &provider_type,
        &system_prompt,
        &user_prompt,
        Some(&combined_instructions)
    )
    .await?;

    spinner.finish_and_clear();

    if print {
        // Print the generated message to stdout and exit
        println!("{}", initial_message);
        return Ok(());
    }

    // Initialize interactive commit process with program name and version
    let mut interactive_commit = InteractiveCommit::new(
        initial_message,
        combined_instructions.clone(),
        crate_name!().to_string(),
        crate_version!().to_string(),
    );

    let config = Arc::new(config);

    // Run the interactive commit process
    let commit_performed = interactive_commit
        .run(move |edited_instructions| {
            let config = Arc::clone(&config);
            let provider_type = provider_type.clone();
            let system_prompt = system_prompt.clone();
            let user_prompt = user_prompt.clone();
            let instructions = edited_instructions.to_string();
            async move {
                get_refined_message(
                    &config,
                    &provider_type,
                    &system_prompt,
                    &user_prompt,
                    Some(&instructions),
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
    preset: Option<String>,
) -> Result<()> {
    log_debug!("Starting 'config' command with provider: {:?}, api_key: {:?}, model: {:?}, param: {:?}, gitmoji: {:?}, instructions: {:?}, token_limit: {:?}, preset: {:?}",
               provider, api_key, model, param, gitmoji, instructions, token_limit, preset);

    let mut config = Config::load()?;
    let mut changes_made = false;

    if let Some(provider) = provider {
        if !get_available_providers()
            .iter()
            .any(|p| p.to_string() == provider)
        {
            return Err(anyhow!("Invalid provider: {}", provider));
        }
        if config.default_provider != provider {
            config.default_provider = provider.clone();
            changes_made = true;
        }
        if !config.providers.contains_key(&provider) {
            config
                .providers
                .insert(provider.clone(), Default::default());
            changes_made = true;
        }
    }

    let provider_config = config.providers.get_mut(&config.default_provider).unwrap();

    if let Some(key) = api_key {
        if provider_config.api_key != key {
            provider_config.api_key = key;
            changes_made = true;
        }
    }
    if let Some(model) = model {
        if provider_config.model != model {
            provider_config.model = model;
            changes_made = true;
        }
    }
    if let Some(params) = param {
        let additional_params = parse_additional_params(&params);
        if provider_config.additional_params != additional_params {
            provider_config.additional_params = additional_params;
            changes_made = true;
        }
    }
    if let Some(use_gitmoji) = gitmoji {
        if config.use_gitmoji != use_gitmoji {
            config.use_gitmoji = use_gitmoji;
            changes_made = true;
        }
    }
    if let Some(instr) = instructions {
        if config.instructions != instr {
            config.instructions = instr;
            changes_made = true;
        }
    }
    if let Some(limit) = token_limit {
        if provider_config.token_limit != Some(limit) {
            provider_config.token_limit = Some(limit);
            changes_made = true;
        }
    }
    if let Some(preset) = preset {
        let preset_library = get_instruction_preset_library();
        if preset_library.get_preset(&preset).is_some() {
            if config.instruction_preset != preset {
                config.instruction_preset = preset;
                changes_made = true;
            }
        } else {
            return Err(anyhow!("Invalid preset: {}", preset));
        }
    }

    if changes_made {
        config.save()?;
        ui::print_success("Configuration updated successfully.");
    }

    ui::print_info(&format!(
        "Current configuration:\nDefault Provider: {}\nUse Gitmoji: {}\nInstructions: {}\nInstruction Preset: {}",
        config.default_provider,
        config.use_gitmoji,
        if config.instructions.is_empty() {
            "None".to_string()
        } else {
            config.instructions.replace('\n', ", ")
        },
        config.instruction_preset
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

pub async fn handle_changelog_command(from: String, to: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let spinner = ui::create_spinner("Generating changelog...");

    let repo_path = env::current_dir()?;
    let to = to.unwrap_or_else(|| "HEAD".to_string());
    let changelog = ChangelogGenerator::generate(&repo_path, &from, &to, &config).await?;

    spinner.finish_and_clear();
    
    // Save changelog to file
    /*let filename = format!("changelog-{}-to-{}.md", from, to);
    
    let file_path = std::path::PathBuf::from(&filename);
    fs::write(&file_path, &changelog)?;
    
    ui::print_success(&format!("Changelog generated and saved to {}", file_path.display()));
    ui::print_info("\nChangelog Preview:");
    */
    println!("{}", "━".repeat(50).bright_purple());
    println!("{}", &changelog);
    println!("{}", "━".repeat(50).bright_purple());

    Ok(())
}

pub async fn handle_release_notes_command(from: String, to: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let spinner = ui::create_spinner("Generating release notes...");

    let repo_path = env::current_dir()?;
    let to = to.unwrap_or_else(|| "HEAD".to_string());
    let release_notes = ReleaseNotesGenerator::generate(&repo_path, &from, &to, &config).await?;

    spinner.finish_and_clear();
    
    // Save release notes to file
    /*let filename = format!("release-notes-{}-to-{}.md", from, to);
    let file_path = PathBuf::from(&filename);
    fs::write(&file_path, &release_notes)?;
    
    ui::print_success(&format!("Release notes generated and saved to {}", file_path.display()));
    ui::print_info("\nRelease Notes Preview:");
    */
    println!("{}", "━".repeat(50).bright_purple());
    println!("{}", &release_notes);
    println!("{}", "━".repeat(50).bright_purple());

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

/// Handle the 'list_presets' command
pub fn handle_list_presets_command() -> Result<()> {
    let preset_library = get_instruction_preset_library();

    println!(
        "{}",
        "\n🔮 Available Instruction Presets 🔮"
            .bright_purple()
            .bold()
    );
    println!("{}", "━".repeat(50).bright_purple());

    let mut presets = preset_library.list_presets();
    presets.sort_by(|a, b| a.0.cmp(b.0)); // Sort alphabetically by key

    let max_key_length = presets
        .iter()
        .map(|(key, _)| key.width())
        .max()
        .unwrap_or(0);

    for (key, preset) in presets {
        println!(
            "{} {:<width$} {}",
            "•".bright_cyan(),
            key.bright_green().bold(),
            preset.name.cyan().italic(),
            width = max_key_length
        );
        println!("  {}", format!("\"{}\"", preset.description).bright_white());
        println!(); // Add a blank line between presets
    }

    println!("{}", "━".repeat(50).bright_purple());
    println!(
        "{}",
        "Use with: git-iris gen --preset <preset-name>"
            .bright_yellow()
            .italic()
    );

    Ok(())
}
