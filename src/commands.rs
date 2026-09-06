use crate::common::CommonParams;
use crate::config::Config;
use crate::instruction_presets::{
    PresetType, get_instruction_preset_library, list_presets_formatted_by_type,
};
use crate::log_debug;
use crate::providers::{Provider, ProviderConfig};
use crate::ui;
use anyhow::Context;
use anyhow::{Result, anyhow};
use colored::Colorize;
use std::collections::HashMap;

/// Helper to get themed colors for terminal output
mod colors {
    use crate::theme;
    use crate::theme::names::tokens;

    pub fn accent_primary() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::ACCENT_PRIMARY);
        (c.r, c.g, c.b)
    }

    pub fn accent_secondary() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::ACCENT_SECONDARY);
        (c.r, c.g, c.b)
    }

    pub fn accent_tertiary() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::ACCENT_TERTIARY);
        (c.r, c.g, c.b)
    }

    pub fn warning() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::WARNING);
        (c.r, c.g, c.b)
    }

    pub fn success() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::SUCCESS);
        (c.r, c.g, c.b)
    }

    pub fn text_secondary() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::TEXT_SECONDARY);
        (c.r, c.g, c.b)
    }

    pub fn text_dim() -> (u8, u8, u8) {
        let c = theme::current().color(tokens::TEXT_DIM);
        (c.r, c.g, c.b)
    }
}

/// Apply common configuration changes to a config object
/// Returns true if any changes were made
///
/// This centralized function handles changes to configuration objects, used by both
/// personal and project configuration commands.
///
/// # Arguments
///
/// * `config` - The configuration object to modify
/// * `common` - Common parameters from command line
/// * `model` - Optional model to set for the selected provider
/// * `token_limit` - Optional token limit to set
/// * `param` - Optional additional parameters to set
/// * `api_key` - Optional API key to set (ignored in project configs)
///
/// # Returns
///
/// Boolean indicating if any changes were made to the configuration
fn apply_config_changes(
    config: &mut Config,
    common: &CommonParams,
    model: Option<String>,
    fast_model: Option<String>,
    subagent_model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
    api_key: Option<String>,
    subagent_timeout: Option<u64>,
    subagent_max_turns: Option<usize>,
) -> anyhow::Result<bool> {
    let mut changes_made = false;

    // Apply common parameters to the config and track if changes were made
    let common_changes = common.apply_to_config(config)?;
    changes_made |= common_changes;

    // Handle provider change - validate and insert if needed
    if let Some(provider_str) = &common.provider {
        changes_made |= ensure_provider_config(config, provider_str)?;
    }

    let provider_config = config
        .providers
        .get_mut(&config.default_provider)
        .context("Could not get default provider")?;

    // Apply API key if provided
    if let Some(key) = api_key
        && provider_config.api_key != key
    {
        provider_config.api_key = key;
        changes_made = true;
    }

    // Apply model change
    if let Some(model) = model
        && provider_config.model != model
    {
        provider_config.model = model;
        changes_made = true;
    }

    // Apply fast model change
    if let Some(model) = subagent_model
        && provider_config.subagent_model.as_ref() != Some(&model)
    {
        provider_config.subagent_model = Some(model);
        changes_made = true;
    }

    if let Some(fast_model) = fast_model
        && provider_config.fast_model != Some(fast_model.clone())
    {
        provider_config.fast_model = Some(fast_model);
        changes_made = true;
    }

    // Apply parameter changes
    if let Some(params) = param {
        let additional_params = parse_additional_params(&params);
        if provider_config.additional_params != additional_params {
            provider_config.additional_params = additional_params;
            changes_made = true;
        }
    }

    // Apply gitmoji setting
    if let Some(use_gitmoji) = common.resolved_gitmoji()
        && config.use_gitmoji != use_gitmoji
    {
        config.use_gitmoji = use_gitmoji;
        changes_made = true;
    }

    if let Some(critic_enabled) = common.resolved_critic()
        && config.critic_enabled != critic_enabled
    {
        config.critic_enabled = critic_enabled;
        changes_made = true;
    }

    // Apply instructions
    if let Some(instr) = &common.instructions
        && config.instructions != *instr
    {
        config.instructions.clone_from(instr);
        changes_made = true;
    }

    // Apply token limit
    if let Some(limit) = token_limit
        && provider_config.token_limit != Some(limit)
    {
        provider_config.token_limit = Some(limit);
        changes_made = true;
    }

    // Apply preset
    if let Some(preset) = &common.preset {
        let preset_library = get_instruction_preset_library();
        if preset_library.get_preset(preset).is_some() {
            if config.instruction_preset != *preset {
                config.instruction_preset.clone_from(preset);
                changes_made = true;
            }
        } else {
            return Err(anyhow!("Invalid preset: {}", preset));
        }
    }

    // Apply subagent limits
    if let Some(timeout) = subagent_timeout
        && config.subagent_timeout_secs != timeout
    {
        config.subagent_timeout_secs = timeout;
        changes_made = true;
    }
    if let Some(max_turns) = subagent_max_turns
        && config.subagent_max_turns != max_turns
    {
        config.subagent_max_turns = max_turns;
        changes_made = true;
    }

    Ok(changes_made)
}

fn ensure_provider_config(config: &mut Config, provider_name: &str) -> anyhow::Result<bool> {
    let provider: Provider = provider_name.parse().map_err(|_| {
        anyhow!(
            "Invalid provider: {}. Available: {}",
            provider_name,
            Provider::all_names().join(", ")
        )
    })?;

    if config.providers.contains_key(provider.name()) {
        return Ok(false);
    }

    config.providers.insert(
        provider.name().to_string(),
        ProviderConfig::with_defaults(provider),
    );
    Ok(true)
}

/// Handle the 'config' command
#[allow(clippy::too_many_lines)]
///
/// # Errors
///
/// Returns an error when configuration loading, validation, or saving fails.
pub fn handle_config_command(
    common: &CommonParams,
    api_key: Option<String>,
    model: Option<String>,
    fast_model: Option<String>,
    subagent_model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
    subagent_timeout: Option<u64>,
    subagent_max_turns: Option<usize>,
) -> anyhow::Result<()> {
    log_debug!(
        "Starting 'config' command with common: {:?}, api_key: {}, model: {:?}, token_limit: {:?}, param: {:?}, subagent_timeout: {:?}, subagent_max_turns: {:?}",
        common,
        if api_key.is_some() {
            "[REDACTED]"
        } else {
            "<none>"
        },
        model,
        token_limit,
        param,
        subagent_timeout,
        subagent_max_turns
    );

    let mut config = Config::load()?;

    // Apply configuration changes
    let changes_made = apply_config_changes(
        &mut config,
        common,
        model,
        fast_model,
        subagent_model,
        token_limit,
        param,
        api_key,
        subagent_timeout,
        subagent_max_turns,
    )?;

    if changes_made {
        config.save()?;
        ui::print_success("Configuration updated successfully.");
        ui::print_newline();
    }

    // Print the configuration with beautiful styling
    print_configuration(&config);

    Ok(())
}

/// Handle printing current project configuration
///
/// Loads and displays the current project configuration if it exists,
/// or shows a message if no project configuration is found.
fn print_project_config() {
    if let Ok(project_config) = Config::load_project_config() {
        ui::print_message(&format!(
            "\n{}",
            "Current project configuration:".bright_cyan().bold()
        ));
        print_configuration(&project_config);
    } else {
        ui::print_message(&format!(
            "\n{}",
            "No project configuration file found.".yellow()
        ));
        ui::print_message("You can create one with the project-config command.");
    }
}

/// Handle the 'project-config' command
///
/// Creates or updates a project-specific configuration file (.irisconfig)
/// in the repository root. Project configurations allow teams to share
/// common settings without sharing sensitive data like API keys.
///
/// # Security
///
/// API keys are never stored in project configuration files, ensuring that
/// sensitive credentials are not accidentally committed to version control.
///
/// # Arguments
///
/// * `common` - Common parameters from command line
/// * `model` - Optional model to set for the selected provider
/// * `token_limit` - Optional token limit to set
/// * `param` - Optional additional parameters to set
/// * `print` - Whether to just print the current project config
///
/// # Returns
///
/// Result indicating success or an error
///
/// # Errors
///
/// Returns an error when project configuration validation or saving fails.
pub fn handle_project_config_command(
    common: &CommonParams,
    model: Option<String>,
    fast_model: Option<String>,
    subagent_model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
    subagent_timeout: Option<u64>,
    subagent_max_turns: Option<usize>,
    print: bool,
) -> anyhow::Result<()> {
    log_debug!(
        "Starting 'project-config' command with common: {:?}, model: {:?}, token_limit: {:?}, param: {:?}, subagent_timeout: {:?}, subagent_max_turns: {:?}, print: {}",
        common,
        model,
        token_limit,
        param,
        subagent_timeout,
        subagent_max_turns,
        print
    );

    println!("\n{}", "✨ Project Configuration".bright_magenta().bold());

    if print {
        print_project_config();
        return Ok(());
    }

    let mut config = Config::load_project_config().unwrap_or_else(|_| Config {
        default_provider: String::new(),
        providers: HashMap::new(),
        use_gitmoji: true,
        instructions: String::new(),
        instruction_preset: String::new(),
        theme: String::new(),
        subagent_timeout_secs: 120,
        subagent_max_turns: 20,
        critic_enabled: true,
        temp_instructions: None,
        temp_preset: None,
        is_project_config: true,
        gitmoji_override: None,
        critic_override: None,
    });

    let mut changes_made = false;

    // Apply provider settings
    let provider_name = apply_provider_settings(
        &mut config,
        common,
        model,
        fast_model,
        subagent_model,
        token_limit,
        param,
        &mut changes_made,
    )?;

    // Apply common settings
    apply_common_settings(
        &mut config,
        common,
        subagent_timeout,
        subagent_max_turns,
        &mut changes_made,
    )?;

    // Display result
    display_project_config_result(&config, changes_made, &provider_name)?;

    Ok(())
}

/// Apply provider-related settings to config
fn apply_provider_settings(
    config: &mut Config,
    common: &CommonParams,
    model: Option<String>,
    fast_model: Option<String>,
    subagent_model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
    changes_made: &mut bool,
) -> anyhow::Result<String> {
    // Apply provider change
    if let Some(provider_str) = &common.provider {
        let provider: Provider = provider_str.parse().map_err(|_| {
            anyhow!(
                "Invalid provider: {}. Available: {}",
                provider_str,
                Provider::all_names().join(", ")
            )
        })?;

        if config.default_provider != provider.name() {
            config.default_provider = provider.name().to_string();
            config
                .providers
                .entry(provider.name().to_string())
                .or_default();
            *changes_made = true;
        }
    }

    // Get provider name to use
    let provider_name = if config.default_provider.is_empty() {
        Provider::default().name().to_string()
    } else {
        config.default_provider.clone()
    };

    // Ensure provider config entry exists if setting model options
    if model.is_some()
        || fast_model.is_some()
        || subagent_model.is_some()
        || token_limit.is_some()
        || param.is_some()
    {
        config.providers.entry(provider_name.clone()).or_default();
    }

    // Apply model settings
    if let Some(m) = model
        && let Some(pc) = config.providers.get_mut(&provider_name)
        && pc.model != m
    {
        pc.model = m;
        *changes_made = true;
    }

    if let Some(model) = subagent_model
        && let Some(pc) = config.providers.get_mut(&provider_name)
        && pc.subagent_model.as_ref() != Some(&model)
    {
        pc.subagent_model = Some(model);
        *changes_made = true;
    }

    if let Some(fm) = fast_model
        && let Some(pc) = config.providers.get_mut(&provider_name)
        && pc.fast_model != Some(fm.clone())
    {
        pc.fast_model = Some(fm);
        *changes_made = true;
    }

    if let Some(limit) = token_limit
        && let Some(pc) = config.providers.get_mut(&provider_name)
        && pc.token_limit != Some(limit)
    {
        pc.token_limit = Some(limit);
        *changes_made = true;
    }

    if let Some(params) = param
        && let Some(pc) = config.providers.get_mut(&provider_name)
    {
        let additional_params = parse_additional_params(&params);
        if pc.additional_params != additional_params {
            pc.additional_params = additional_params;
            *changes_made = true;
        }
    }

    Ok(provider_name)
}

/// Apply common settings (gitmoji, instructions, preset, timeout)
fn apply_common_settings(
    config: &mut Config,
    common: &CommonParams,
    subagent_timeout: Option<u64>,
    subagent_max_turns: Option<usize>,
    changes_made: &mut bool,
) -> anyhow::Result<()> {
    if let Some(use_gitmoji) = common.resolved_gitmoji()
        && config.use_gitmoji != use_gitmoji
    {
        config.use_gitmoji = use_gitmoji;
        *changes_made = true;
    }

    if let Some(critic_enabled) = common.resolved_critic()
        && config.critic_enabled != critic_enabled
    {
        config.critic_enabled = critic_enabled;
        *changes_made = true;
    }

    if let Some(instr) = &common.instructions
        && config.instructions != *instr
    {
        config.instructions.clone_from(instr);
        *changes_made = true;
    }

    if let Some(preset) = &common.preset {
        let preset_library = get_instruction_preset_library();
        if preset_library.get_preset(preset).is_some() {
            if config.instruction_preset != *preset {
                config.instruction_preset.clone_from(preset);
                *changes_made = true;
            }
        } else {
            return Err(anyhow!("Invalid preset: {}", preset));
        }
    }

    if let Some(timeout) = subagent_timeout
        && config.subagent_timeout_secs != timeout
    {
        config.subagent_timeout_secs = timeout;
        *changes_made = true;
    }
    if let Some(max_turns) = subagent_max_turns
        && config.subagent_max_turns != max_turns
    {
        config.subagent_max_turns = max_turns;
        *changes_made = true;
    }

    Ok(())
}

/// Display the result of project config command
fn display_project_config_result(
    config: &Config,
    changes_made: bool,
    _provider_name: &str,
) -> anyhow::Result<()> {
    if changes_made {
        config.save_as_project_config()?;
        ui::print_success("Project configuration created/updated successfully.");
        println!();
        println!(
            "{}",
            "Note: API keys are never stored in project configuration files."
                .yellow()
                .italic()
        );
        println!();
        println!("{}", "Current project configuration:".bright_cyan().bold());
        print_configuration(config);
    } else {
        println!("{}", "No changes made to project configuration.".yellow());
        println!();

        if let Ok(project_config) = Config::load_project_config() {
            println!("{}", "Current project configuration:".bright_cyan().bold());
            print_configuration(&project_config);
        } else {
            println!("{}", "No project configuration exists yet.".bright_yellow());
            println!(
                "{}",
                "Use this command with options like --model or --provider to create one."
                    .bright_white()
            );
        }
    }
    Ok(())
}

/// Display the configuration with `SilkCircuit` styling
#[allow(clippy::too_many_lines)]
fn print_configuration(config: &Config) {
    let purple = colors::accent_primary();
    let cyan = colors::accent_secondary();
    let green = colors::success();
    let dim = colors::text_secondary();
    let dim_sep = colors::text_dim();

    println!();
    println!(
        "{}  {}  {}",
        "━━━".truecolor(purple.0, purple.1, purple.2),
        "IRIS CONFIGURATION"
            .truecolor(cyan.0, cyan.1, cyan.2)
            .bold(),
        "━━━".truecolor(purple.0, purple.1, purple.2)
    );
    println!();

    // Global Settings
    print_section_header("GLOBAL");

    print_config_row("Provider", &config.default_provider, cyan, true);

    // Theme
    let theme = crate::theme::current();
    print_config_row("Theme", &theme.meta.name, purple, false);

    print_config_row(
        "Gitmoji",
        if config.use_gitmoji {
            "enabled"
        } else {
            "disabled"
        },
        if config.use_gitmoji { green } else { dim },
        false,
    );
    print_config_row("Preset", &config.instruction_preset, dim, false);
    print_config_row(
        "Critic",
        if config.critic_enabled {
            "enabled"
        } else {
            "disabled"
        },
        if config.critic_enabled { green } else { dim },
        false,
    );
    print_config_row(
        "Timeout",
        &format!("{}s", config.subagent_timeout_secs),
        dim,
        false,
    );
    print_config_row(
        "Subagent Max Turns",
        &config.subagent_max_turns.to_string(),
        dim,
        false,
    );

    // Config file paths
    if let Ok(config_path) = Config::get_personal_config_path() {
        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default();
        let path_str = config_path.to_string_lossy().to_string();
        let path_display = if home.is_empty() {
            path_str
        } else {
            path_str.replace(&home, "~")
        };
        print_config_row("Config", &path_display, dim, false);
    }

    // Project config status
    if let Ok(project_path) = Config::get_project_config_path()
        && project_path.exists()
    {
        print_config_row("Project", ".irisconfig ✓", green, false);
    }

    // Custom Instructions (if any)
    if !config.instructions.is_empty() {
        println!();
        print_section_header("INSTRUCTIONS");
        // Truncate long instructions for display
        let preview: String = config
            .instructions
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        for line in preview.lines() {
            println!("    {}", line.truecolor(dim.0, dim.1, dim.2).italic());
        }
        let total_lines = config.instructions.lines().count();
        if total_lines > 3 {
            println!(
                "    {}",
                format!("… ({} more lines)", total_lines - 3)
                    .truecolor(dim_sep.0, dim_sep.1, dim_sep.2)
                    .italic()
            );
        }
    }

    // Show ALL providers — active provider first, then rest alphabetically
    let mut provider_names: Vec<String> =
        Provider::ALL.iter().map(|p| p.name().to_string()).collect();
    provider_names.sort();
    // Move active provider to front
    if let Some(pos) = provider_names
        .iter()
        .position(|n| n == &config.default_provider)
    {
        let active = provider_names.remove(pos);
        provider_names.insert(0, active);
    }

    for provider_name in &provider_names {
        println!();
        print_provider_section(config, provider_name);
    }

    println!();
    println!(
        "{}",
        "─".repeat(44).truecolor(dim_sep.0, dim_sep.1, dim_sep.2)
    );
    println!();
}

/// Print a single provider section with all its details
fn print_provider_section(config: &Config, provider_name: &str) {
    let cyan = colors::accent_secondary();
    let coral = colors::accent_tertiary();
    let yellow = colors::warning();
    let green = colors::success();
    let dim = colors::text_secondary();
    let error_red: (u8, u8, u8) = (255, 99, 99);

    let is_active = provider_name == config.default_provider;
    let provider: Option<Provider> = provider_name.parse().ok();

    let header = if is_active {
        format!("{} ✦", provider_name.to_uppercase())
    } else {
        provider_name.to_uppercase()
    };
    print_section_header(&header);

    let provider_config = config.providers.get(provider_name);

    // Model (resolve effective model)
    let model = provider_config
        .and_then(|pc| provider.map(|p| pc.effective_model(p).to_string()))
        .or_else(|| provider.map(|p| p.default_model().to_string()))
        .unwrap_or_default();
    print_config_row("Model", &model, cyan, is_active);

    // Fast Model (resolve effective)
    let fast_model = provider_config
        .and_then(|pc| provider.map(|p| pc.effective_fast_model(p).to_string()))
        .or_else(|| provider.map(|p| p.default_fast_model().to_string()))
        .unwrap_or_default();
    print_config_row("Fast Model", &fast_model, dim, false);
    let subagent_model = provider_config
        .and_then(|pc| provider.map(|p| pc.effective_subagent_model(p)))
        .unwrap_or(&model);
    print_config_row("Subagent Model", subagent_model, dim, false);

    // Context window
    if let Some(p) = provider {
        let effective_limit =
            provider_config.map_or_else(|| p.context_window(), |pc| pc.effective_token_limit(p));
        let limit_str = format_token_count(effective_limit);
        let is_custom = provider_config.and_then(|pc| pc.token_limit).is_some();
        if is_custom {
            print_config_row("Context", &format!("{limit_str} (custom)"), coral, false);
        } else {
            print_config_row("Context", &limit_str, dim, false);
        }
    }

    // API Key status
    if let Some(p) = provider {
        let has_config_key = provider_config.is_some_and(ProviderConfig::has_api_key);
        let has_env_key = std::env::var(p.api_key_env()).is_ok();
        let env_var = p.api_key_env();

        let (status, status_color) = if has_config_key {
            // Safe: has_config_key guarantees provider_config is Some with a key
            let key = &provider_config.expect("checked above").api_key;
            let masked = mask_api_key(key);
            (format!("✓ {masked}"), green)
        } else if has_env_key {
            (format!("✓ ${env_var}"), green)
        } else {
            (format!("✗ not set  →  ${env_var}"), error_red)
        };
        print_config_row("API Key", &status, status_color, false);

        // Show format warning if key exists but has bad format
        let key_value = if has_config_key {
            provider_config.map(|pc| pc.api_key.clone())
        } else if has_env_key {
            std::env::var(p.api_key_env()).ok()
        } else {
            None
        };
        if let Some(ref key) = key_value
            && let Err(warning) = p.validate_api_key_format(key)
        {
            println!(
                "              {}",
                format!("⚠ {warning}").truecolor(yellow.0, yellow.1, yellow.2)
            );
        }
    }

    // Additional Parameters
    if let Some(pc) = provider_config
        && !pc.additional_params.is_empty()
    {
        for (key, value) in &pc.additional_params {
            print_config_row(key, value, dim, false);
        }
    }
}

/// Format a token count in human-readable form (128K, 200K, 1M)
fn format_token_count(count: usize) -> String {
    if count >= 1_000_000 && count.is_multiple_of(1_000_000) {
        format!("{}M tokens", count / 1_000_000)
    } else if count >= 1_000 {
        format!("{}K tokens", count / 1_000)
    } else {
        format!("{count} tokens")
    }
}

/// Mask an API key for display, showing only prefix and last 4 chars
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "••••".to_string();
    }
    // Show the prefix (e.g. "sk-ant-") and last 4 chars
    let prefix_end = key.find('-').map_or(4, |i| {
        // Find the last hyphen in the prefix portion (first 12 chars)
        key[..12.min(key.len())].rfind('-').map_or(i + 1, |j| j + 1)
    });
    let prefix = &key[..prefix_end.min(key.len())];
    let suffix = &key[key.len() - 4..];
    format!("{prefix}••••{suffix}")
}

/// Print a section header in `SilkCircuit` style
fn print_section_header(name: &str) {
    let purple = colors::accent_primary();
    let dim_sep = colors::text_dim();
    println!(
        "{} {} {}",
        "─".truecolor(purple.0, purple.1, purple.2),
        name.truecolor(purple.0, purple.1, purple.2).bold(),
        "─"
            .repeat(30 - name.len().min(28))
            .truecolor(dim_sep.0, dim_sep.1, dim_sep.2)
    );
}

/// Print a config row with label and value
fn print_config_row(label: &str, value: &str, value_color: (u8, u8, u8), highlight: bool) {
    let dim = colors::text_secondary();
    let label_styled = format!("{label:>12}").truecolor(dim.0, dim.1, dim.2);

    let value_styled = if highlight {
        value
            .truecolor(value_color.0, value_color.1, value_color.2)
            .bold()
    } else {
        value.truecolor(value_color.0, value_color.1, value_color.2)
    };

    println!("{label_styled}  {value_styled}");
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

/// Handle the '`list_presets`' command
///
/// # Errors
///
/// Returns an error if preset formatting fails in the future; currently always succeeds.
pub fn handle_list_presets_command() -> Result<()> {
    let library = get_instruction_preset_library();

    // Get different categories of presets
    let both_presets = list_presets_formatted_by_type(&library, Some(PresetType::Both));
    let commit_only_presets = list_presets_formatted_by_type(&library, Some(PresetType::Commit));
    let review_only_presets = list_presets_formatted_by_type(&library, Some(PresetType::Review));

    println!(
        "{}",
        "\nGit-Iris Instruction Presets\n".bright_magenta().bold()
    );

    println!(
        "{}",
        "General Presets (usable for both commit and review):"
            .bright_cyan()
            .bold()
    );
    println!("{both_presets}\n");

    if !commit_only_presets.is_empty() {
        println!("{}", "Commit-specific Presets:".bright_green().bold());
        println!("{commit_only_presets}\n");
    }

    if !review_only_presets.is_empty() {
        println!("{}", "Review-specific Presets:".bright_blue().bold());
        println!("{review_only_presets}\n");
    }

    println!("{}", "Usage:".bright_yellow().bold());
    println!("  git-iris gen --preset <preset-key>");
    println!("  git-iris review --preset <preset-key>");
    println!("\nPreset types: [B] = Both commands, [C] = Commit only, [R] = Review only");

    Ok(())
}

/// Marker comment embedded in hooks installed by git-iris
const HOOK_MARKER: &str = "# Installed by git-iris";

/// Handle the `hook` command - install or uninstall the prepare-commit-msg hook
///
/// # Errors
///
/// Returns an error when hook installation or removal fails.
pub fn handle_hook_command(action: &crate::cli::HookAction) -> Result<()> {
    match action {
        crate::cli::HookAction::Install { force } => handle_hook_install(*force),
        crate::cli::HookAction::Uninstall => handle_hook_uninstall(),
    }
}

/// Install the prepare-commit-msg hook
fn handle_hook_install(force: bool) -> Result<()> {
    use std::fs;

    let hook_dir = find_git_hooks_dir()?;
    let hook_path = hook_dir.join("prepare-commit-msg");

    // Reject symlinks to prevent redirect attacks
    if hook_path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        anyhow::bail!(
            "Hook path is a symlink — refusing to write. Remove it manually: {}",
            hook_path.display()
        );
    }

    // Check for existing hook
    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path).context("Failed to read existing hook")?;

        if existing.contains(HOOK_MARKER) {
            let (r, g, b) = colors::success();
            println!(
                "{}",
                "✨ Git-iris hook is already installed.".truecolor(r, g, b)
            );
            return Ok(());
        }

        if !force {
            let (r, g, b) = colors::warning();
            println!(
                "{}",
                "⚠️  A prepare-commit-msg hook already exists and was not installed by git-iris."
                    .truecolor(r, g, b)
            );
            println!("{}", "   Use --force to overwrite it.".truecolor(r, g, b));
            return Ok(());
        }
    }

    let hook_content = format!(
        "#!/bin/sh\n{HOOK_MARKER}\n# Generates an AI commit message using git-iris\nexec git-iris gen --print > \"$1\"\n"
    );

    fs::write(&hook_path, hook_content).context("Failed to write hook file")?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))
            .context("Failed to set hook permissions")?;
    }

    let (r, g, b) = colors::success();
    println!(
        "{}",
        "✨ prepare-commit-msg hook installed successfully!".truecolor(r, g, b)
    );
    println!(
        "   {}",
        format!("Hook path: {}", hook_path.display()).truecolor(r, g, b)
    );
    println!(
        "   {}",
        "AI commit messages will be generated automatically when you run 'git commit'."
            .truecolor(r, g, b)
    );

    Ok(())
}

/// Uninstall the prepare-commit-msg hook
fn handle_hook_uninstall() -> Result<()> {
    use std::fs;

    let hook_dir = find_git_hooks_dir()?;
    let hook_path = hook_dir.join("prepare-commit-msg");

    // Reject symlinks to prevent redirect attacks
    if hook_path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        anyhow::bail!(
            "Hook path is a symlink — refusing to remove. Delete it manually: {}",
            hook_path.display()
        );
    }

    if !hook_path.exists() {
        let (r, g, b) = colors::warning();
        println!("{}", "No prepare-commit-msg hook found.".truecolor(r, g, b));
        return Ok(());
    }

    let content = fs::read_to_string(&hook_path).context("Failed to read hook file")?;

    if !content.contains(HOOK_MARKER) {
        let (r, g, b) = colors::warning();
        println!(
            "{}",
            "⚠️  The existing prepare-commit-msg hook was not installed by git-iris."
                .truecolor(r, g, b)
        );
        println!(
            "   {}",
            "Refusing to remove it. Delete it manually if needed.".truecolor(r, g, b)
        );
        return Ok(());
    }

    fs::remove_file(&hook_path).context("Failed to remove hook file")?;

    let (r, g, b) = colors::success();
    println!(
        "{}",
        "✨ prepare-commit-msg hook uninstalled successfully.".truecolor(r, g, b)
    );

    Ok(())
}

/// Find the hooks directory using libgit2.
///
/// Uses `repo.path()` which resolves correctly for both normal repos and
/// worktrees (where `.git` is a file, not a directory). Also respects
/// `core.hooksPath` when configured.
fn find_git_hooks_dir() -> Result<std::path::PathBuf> {
    use crate::git::GitRepo;

    let repo_root = GitRepo::get_repo_root()
        .context("Not in a Git repository. Run this command from within a Git repository.")?;

    let repo = git2::Repository::open(&repo_root).context("Failed to open Git repository")?;

    // Check for core.hooksPath override first
    let hooks_dir = repo
        .config()
        .ok()
        .and_then(|cfg| cfg.get_path("core.hooksPath").ok())
        .unwrap_or_else(|| repo.path().join("hooks"));

    // Create hooks dir if it doesn't exist
    if !hooks_dir.exists() {
        std::fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory")?;
    }

    Ok(hooks_dir)
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
