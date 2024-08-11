use crate::changelog::{ChangelogGenerator, ReleaseNotesGenerator};
use crate::common::{CommonParams, DetailLevel};
use crate::config::Config;
use crate::context::format_commit_message;
use crate::instruction_presets::get_instruction_preset_library;
use crate::llm_providers::{get_available_providers, LLMProviderType};
use crate::log_debug;
use crate::messages;
use crate::service::GitIrisService;
use crate::tui::run_tui_commit;
use crate::ui;
use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

/// Handle the 'gen' command
pub async fn handle_gen_command(
    common: CommonParams,
    auto_commit: bool,
    use_gitmoji: bool,
    print: bool,
) -> Result<()> {
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;
    let current_dir = std::env::current_dir()?;

    let provider_type = LLMProviderType::from_str(&config.default_provider)?;

    let service = Arc::new(GitIrisService::new(
        config.clone(),
        current_dir.clone(),
        provider_type.clone(),
        use_gitmoji && config.use_gitmoji,
    ));

    // Check environment prerequisites
    if let Err(e) = service.check_environment() {
        ui::print_error(&format!("Error: {}", e));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info("2. You are running this command from within a Git repository.");
        ui::print_info("3. You have set up your configuration using 'git-iris config'.");
        return Err(e);
    }

    let git_info = service.get_git_info().await?;

    if git_info.staged_files.is_empty() {
        ui::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());
    let preset_str = common.preset.as_deref().unwrap_or("");

    // Create and start the spinner
    let spinner = ui::create_spinner("");
    let random_message = messages::get_random_message();
    spinner.set_message(random_message.text);

    // Generate an initial message
    let initial_message = service
        .generate_message(preset_str, &effective_instructions)
        .await?;

    // Stop the spinner
    spinner.finish_and_clear();

    if print {
        println!("{}", format_commit_message(&initial_message));
        return Ok(());
    }

    if auto_commit {
        service.perform_commit(&format_commit_message(&initial_message))?;
        println!(
            "🌟 Commit created with message: {}",
            format_commit_message(&initial_message)
        );
        return Ok(());
    }

    run_tui_commit(
        vec![initial_message],
        effective_instructions,
        String::from(preset_str),
        git_info.user_name,
        git_info.user_email,
        service,
        config.use_gitmoji,
    )
    .await?;

    Ok(())
}

/// Handle the 'config' command
pub fn handle_config_command(
    common: CommonParams,
    api_key: Option<String>,
    model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
) -> Result<()> {
    log_debug!("Starting 'config' command with common: {:?}, api_key: {:?}, model: {:?}, token_limit: {:?}, param: {:?}",
               common, api_key, model, token_limit, param);

    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;
    let mut changes_made = false;

    if let Some(provider) = common.provider {
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
    if let Some(use_gitmoji) = common.gitmoji {
        if config.use_gitmoji != use_gitmoji {
            config.use_gitmoji = use_gitmoji;
            changes_made = true;
        }
    }
    if let Some(instr) = common.instructions {
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
    if let Some(preset) = common.preset {
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


