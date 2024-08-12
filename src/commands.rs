use crate::common::CommonParams;
use crate::config::Config;
use crate::instruction_presets::get_instruction_preset_library;
use crate::llm_providers::get_available_providers;
use crate::log_debug;
use crate::ui;
use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;

use unicode_width::UnicodeWidthStr;

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
