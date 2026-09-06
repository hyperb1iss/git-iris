//! Configuration management for Git-Iris.
//!
//! Handles personal config (~/.config/git-iris/config.toml) and
//! per-project config (.irisconfig) with proper layering.

use crate::git::GitRepo;
use crate::instruction_presets::get_instruction_preset_library;
use crate::log_debug;
use crate::providers::{Provider, ProviderConfig};

use anyhow::{Context, Result, anyhow};
use dirs::{config_dir, home_dir};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Project configuration filename
pub const PROJECT_CONFIG_FILENAME: &str = ".irisconfig";

/// Main configuration structure
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    /// Default LLM provider
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub default_provider: String,
    /// Provider-specific configurations (keyed by provider name)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub providers: HashMap<String, ProviderConfig>,
    /// Use gitmoji in commit messages
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub use_gitmoji: bool,
    /// Saved custom instructions used as pull request description defaults
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub instructions: String,
    /// Instruction preset name
    #[serde(default = "default_preset", skip_serializing_if = "is_default_preset")]
    pub instruction_preset: String,
    /// Theme name (empty = default `SilkCircuit` Neon)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub theme: String,
    /// Timeout in seconds for parallel subagent tasks (default: 120)
    #[serde(
        default = "default_subagent_timeout",
        skip_serializing_if = "is_default_subagent_timeout"
    )]
    pub subagent_timeout_secs: u64,
    /// Turn budget for parallel subagent tasks (default: 20)
    #[serde(
        default = "default_subagent_max_turns",
        skip_serializing_if = "is_default_subagent_max_turns"
    )]
    pub subagent_max_turns: usize,
    /// Run a critic verification pass after generated artifacts (default: true)
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub critic_enabled: bool,
    /// Runtime-only: temporary instructions for the current operation
    #[serde(skip)]
    pub temp_instructions: Option<String>,
    /// Runtime-only: temporary preset override
    #[serde(skip)]
    pub temp_preset: Option<String>,
    /// Runtime-only: flag if loaded from project config
    #[serde(skip)]
    pub is_project_config: bool,
    /// Runtime-only: whether gitmoji was explicitly set via CLI (None = use style detection)
    #[serde(skip)]
    pub gitmoji_override: Option<bool>,
    /// Runtime-only: whether critic was explicitly set via CLI
    #[serde(skip)]
    pub critic_override: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_true(val: &bool) -> bool {
    *val
}

fn default_preset() -> String {
    "default".to_string()
}

fn is_default_preset(val: &str) -> bool {
    val.is_empty() || val == "default"
}

fn default_subagent_timeout() -> u64 {
    120 // 2 minutes
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_subagent_timeout(val: &u64) -> bool {
    *val == 120
}

fn default_subagent_max_turns() -> usize {
    20
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_subagent_max_turns(val: &usize) -> bool {
    *val == 20
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        for provider in Provider::ALL {
            providers.insert(
                provider.name().to_string(),
                ProviderConfig::with_defaults(*provider),
            );
        }

        Self {
            default_provider: Provider::default().name().to_string(),
            providers,
            use_gitmoji: true,
            instructions: String::new(),
            instruction_preset: default_preset(),
            theme: String::new(),
            subagent_timeout_secs: default_subagent_timeout(),
            subagent_max_turns: default_subagent_max_turns(),
            critic_enabled: true,
            temp_instructions: None,
            temp_preset: None,
            is_project_config: false,
            gitmoji_override: None,
            critic_override: None,
        }
    }
}

impl Config {
    /// Load configuration (personal + project overlay)
    ///
    /// # Errors
    ///
    /// Returns an error when personal or project configuration cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let config_path = Self::get_personal_config_path()?;
        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let parsed: Self = toml::from_str(&content)?;
            let (migrated, needs_save) = Self::migrate_if_needed(parsed);
            if needs_save && let Err(e) = migrated.save() {
                log_debug!("Failed to save migrated config: {}", e);
            }
            migrated
        } else {
            Self::default()
        };

        // Overlay project config if available
        if let Ok((project_config, project_source)) = Self::load_project_config_with_source() {
            config.merge_loaded_project_config(project_config, &project_source);
        }

        log_debug!(
            "Configuration loaded (provider: {}, gitmoji: {})",
            config.default_provider,
            config.use_gitmoji
        );
        Ok(config)
    }

    /// Load project-specific configuration
    ///
    /// # Errors
    ///
    /// Returns an error when the project configuration file is missing or invalid.
    pub fn load_project_config() -> Result<Self> {
        let (config, _) = Self::load_project_config_with_source()?;
        Ok(config)
    }

    fn load_project_config_with_source() -> Result<(Self, toml::Value)> {
        let config_path = Self::get_project_config_path()?;
        if !config_path.exists() {
            return Err(anyhow!("Project configuration file not found"));
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let project_source = toml::from_str(&content).with_context(|| {
            format!(
                "Invalid {} format. Check for syntax errors.",
                PROJECT_CONFIG_FILENAME
            )
        })?;

        let mut config: Self = toml::from_str(&content).with_context(|| {
            format!(
                "Invalid {} format. Check for syntax errors.",
                PROJECT_CONFIG_FILENAME
            )
        })?;

        config.is_project_config = true;
        Ok((config, project_source))
    }

    /// Get path to project config file
    ///
    /// # Errors
    ///
    /// Returns an error when the current repository root cannot be resolved.
    pub fn get_project_config_path() -> Result<PathBuf> {
        let repo_root = GitRepo::get_repo_root()?;
        Ok(repo_root.join(PROJECT_CONFIG_FILENAME))
    }

    /// Merge project config into this config (project takes precedence, but never API keys)
    pub fn merge_with_project_config(&mut self, project_config: Self) {
        log_debug!("Merging with project configuration");

        // Override default provider if set
        if !project_config.default_provider.is_empty()
            && project_config.default_provider != Provider::default().name()
        {
            self.default_provider = project_config.default_provider;
        }

        // Merge provider configs (never override API keys from project config)
        for (provider_name, proj_config) in project_config.providers {
            let entry = self.providers.entry(provider_name).or_default();

            if !proj_config.model.is_empty() {
                entry.model = proj_config.model;
            }
            if proj_config.subagent_model.is_some() {
                entry.subagent_model.clone_from(&proj_config.subagent_model);
            }
            if proj_config.fast_model.is_some() {
                entry.fast_model = proj_config.fast_model;
            }
            if proj_config.token_limit.is_some() {
                entry.token_limit = proj_config.token_limit;
            }
            entry
                .additional_params
                .extend(proj_config.additional_params);
        }

        // Override other settings
        self.use_gitmoji = project_config.use_gitmoji;
        self.instructions = project_config.instructions;

        if project_config.instruction_preset != default_preset() {
            self.instruction_preset = project_config.instruction_preset;
        }

        // Theme override
        if !project_config.theme.is_empty() {
            self.theme = project_config.theme;
        }

        // Subagent timeout override
        if project_config.subagent_timeout_secs != default_subagent_timeout() {
            self.subagent_timeout_secs = project_config.subagent_timeout_secs;
        }
        if project_config.subagent_max_turns != default_subagent_max_turns() {
            self.subagent_max_turns = project_config.subagent_max_turns;
        }
    }

    fn merge_loaded_project_config(&mut self, project_config: Self, project_source: &toml::Value) {
        log_debug!("Merging loaded project configuration with explicit field tracking");

        self.merge_project_provider_config(&project_config);

        if Self::project_config_has_key(project_source, "default_provider") {
            self.default_provider = project_config.default_provider;
        }
        if Self::project_config_has_key(project_source, "use_gitmoji") {
            self.use_gitmoji = project_config.use_gitmoji;
        }
        if Self::project_config_has_key(project_source, "instructions") {
            self.instructions = project_config.instructions;
        }
        if Self::project_config_has_key(project_source, "instruction_preset") {
            self.instruction_preset = project_config.instruction_preset;
        }
        if Self::project_config_has_key(project_source, "theme") {
            self.theme = project_config.theme;
        }
        if Self::project_config_has_key(project_source, "subagent_timeout_secs") {
            self.subagent_timeout_secs = project_config.subagent_timeout_secs;
        }
        if Self::project_config_has_key(project_source, "subagent_max_turns") {
            self.subagent_max_turns = project_config.subagent_max_turns;
        }
        if Self::project_config_has_key(project_source, "critic_enabled") {
            self.critic_enabled = project_config.critic_enabled;
        }
    }

    fn merge_project_provider_config(&mut self, project_config: &Self) {
        for (provider_name, proj_config) in &project_config.providers {
            let entry = self.providers.entry(provider_name.clone()).or_default();

            if !proj_config.model.is_empty() {
                proj_config.model.clone_into(&mut entry.model);
            }
            if proj_config.subagent_model.is_some() {
                entry.subagent_model.clone_from(&proj_config.subagent_model);
            }
            if proj_config.fast_model.is_some() {
                entry.fast_model.clone_from(&proj_config.fast_model);
            }
            if proj_config.token_limit.is_some() {
                entry.token_limit = proj_config.token_limit;
            }
            entry
                .additional_params
                .extend(proj_config.additional_params.clone());
        }
    }

    fn project_config_has_key(project_source: &toml::Value, key: &str) -> bool {
        project_source
            .as_table()
            .is_some_and(|table| table.contains_key(key))
    }

    /// Migrate older config formats. Pure — never touches the filesystem.
    ///
    /// Returns the (possibly updated) config and a flag indicating whether any
    /// migration actually happened. Callers that loaded from disk (i.e. `load`)
    /// are responsible for persisting the migrated form; tests and other
    /// in-memory users can ignore the flag. Keeping this pure stops test
    /// fixtures from clobbering the user's real config file.
    fn migrate_if_needed(mut config: Self) -> (Self, bool) {
        let mut migrated = false;

        for (legacy, canonical) in [("claude", "anthropic"), ("gemini", "google")] {
            if let Some(legacy_config) = config.providers.remove(legacy) {
                log_debug!("Migrating '{legacy}' provider to '{canonical}'");

                if config.providers.contains_key(canonical) {
                    log_debug!(
                        "Keeping existing '{canonical}' config and dropping legacy '{legacy}' entry"
                    );
                } else {
                    config
                        .providers
                        .insert(canonical.to_string(), legacy_config);
                }

                migrated = true;
            }

            if config.default_provider.eq_ignore_ascii_case(legacy) {
                config.default_provider = canonical.to_string();
                migrated = true;
            }
        }

        (config, migrated)
    }

    /// Save configuration to personal config file
    ///
    /// # Errors
    ///
    /// Returns an error when the personal configuration file cannot be serialized or written.
    pub fn save(&self) -> Result<()> {
        if self.is_project_config {
            return Ok(());
        }

        let config_path = Self::get_personal_config_path()?;
        let content = toml::to_string_pretty(self)?;
        Self::write_config_file(&config_path, &content)?;
        log_debug!("Configuration saved");
        Ok(())
    }

    /// Save as project-specific configuration (strips API keys)
    ///
    /// # Errors
    ///
    /// Returns an error when the project configuration file cannot be serialized or written.
    pub fn save_as_project_config(&self) -> Result<()> {
        let config_path = Self::get_project_config_path()?;

        let mut project_config = self.clone();
        project_config.is_project_config = true;

        // Strip API keys for security
        for provider_config in project_config.providers.values_mut() {
            provider_config.api_key.clear();
        }

        let content = toml::to_string_pretty(&project_config)?;
        Self::write_config_file(&config_path, &content)?;
        Ok(())
    }

    /// Write content to a config file with restricted permissions.
    ///
    /// On Unix, creates a temp file with 0o600 permissions first, writes content,
    /// then renames into place — so the target path is never world-readable.
    /// Warns (via stderr) if permission hardening fails rather than silently ignoring.
    fn write_config_file(path: &Path, content: &str) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            // Write to a sibling temp file so rename is atomic on the same filesystem
            let tmp_path = path.with_extension("tmp");
            fs::write(&tmp_path, content)?;
            if let Err(e) = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600)) {
                eprintln!(
                    "Warning: Could not restrict config permissions on {}: {e}",
                    tmp_path.display()
                );
            }
            fs::rename(&tmp_path, path)?;
        }

        #[cfg(not(unix))]
        {
            fs::write(path, content)?;
        }

        Ok(())
    }

    /// Resolve the directory that should hold `config.toml`.
    ///
    /// Precedence:
    /// 1. `$XDG_CONFIG_HOME/git-iris` when the env var is set and non-empty.
    /// 2. `~/Library/Application Support/git-iris` on macOS **only** when a
    ///    config already exists there — this keeps pre-XDG installs working.
    /// 3. `$HOME/.config/git-iris` — the XDG-style default that lines up with
    ///    how `gh`, `neovim`, `bat`, `ripgrep`, `helix`, `starship`, and the
    ///    rest of the modern CLI ecosystem behave on macOS.
    /// 4. `dirs::config_dir()/git-iris` as a last-resort fallback when `$HOME`
    ///    is unreachable (should only happen in exotic sandboxes).
    ///
    /// This function is pure — filesystem probing for the legacy macOS path
    /// happens in `get_personal_config_path` so the resolver stays easy to
    /// unit-test with synthetic inputs.
    fn resolve_personal_config_dir(
        xdg_config_home: Option<PathBuf>,
        home_dir: Option<PathBuf>,
        platform_config_dir: Option<PathBuf>,
        legacy_macos_config_exists: bool,
    ) -> Result<PathBuf> {
        if let Some(xdg) = xdg_config_home.filter(|path| !path.as_os_str().is_empty()) {
            return Ok(xdg.join("git-iris"));
        }

        if legacy_macos_config_exists && let Some(platform) = platform_config_dir.clone() {
            return Ok(platform.join("git-iris"));
        }

        if let Some(home) = home_dir {
            return Ok(home.join(".config").join("git-iris"));
        }

        platform_config_dir
            .map(|p| p.join("git-iris"))
            .ok_or_else(|| anyhow!("Unable to determine config directory"))
    }

    /// Get path to personal config file
    ///
    /// # Errors
    ///
    /// Returns an error when the config directory cannot be resolved or created.
    pub fn get_personal_config_path() -> Result<PathBuf> {
        let platform_dir = config_dir();

        // Only probe the legacy macOS location on macOS. On every other
        // platform `dirs::config_dir()` already maps to `$HOME/.config` (or an
        // equivalent), so treating the existence check as macOS-only avoids
        // falsely flagging a Linux user's `~/.config/git-iris` as "legacy".
        let legacy_macos_config_exists = cfg!(target_os = "macos")
            && platform_dir
                .as_ref()
                .is_some_and(|dir| dir.join("git-iris").join("config.toml").exists());

        let mut path = Self::resolve_personal_config_dir(
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            home_dir(),
            platform_dir,
            legacy_macos_config_exists,
        )?;
        fs::create_dir_all(&path)?;
        path.push("config.toml");
        Ok(path)
    }

    /// Check environment prerequisites
    ///
    /// # Errors
    ///
    /// Returns an error when the current working directory is not inside a Git repository.
    pub fn check_environment(&self) -> Result<()> {
        if !GitRepo::is_inside_work_tree()? {
            return Err(anyhow!(
                "Not in a Git repository. Please run this command from within a Git repository."
            ));
        }
        Ok(())
    }

    /// Set temporary instructions for this session
    pub fn set_temp_instructions(&mut self, instructions: Option<String>) {
        self.temp_instructions = instructions;
    }

    /// Set temporary preset for this session
    pub fn set_temp_preset(&mut self, preset: Option<String>) {
        self.temp_preset = preset;
    }

    /// Get effective preset name (temp overrides saved)
    #[must_use]
    pub fn get_effective_preset_name(&self) -> &str {
        self.temp_preset
            .as_deref()
            .unwrap_or(&self.instruction_preset)
    }

    /// Get effective instructions (combines preset + custom)
    #[must_use]
    pub fn get_effective_instructions(&self) -> String {
        let preset_library = get_instruction_preset_library();
        let preset_instructions = self
            .temp_preset
            .as_ref()
            .or(Some(&self.instruction_preset))
            .and_then(|p| preset_library.get_preset(p))
            .map(|p| p.instructions.clone())
            .unwrap_or_default();

        let custom = self
            .temp_instructions
            .as_ref()
            .unwrap_or(&self.instructions);

        format!("{preset_instructions}\n\n{custom}")
            .trim()
            .to_string()
    }

    /// Update configuration with new values
    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    ///
    /// # Errors
    ///
    /// Returns an error when the provider is invalid or the provider config cannot be updated.
    pub fn update(
        &mut self,
        provider: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
        fast_model: Option<String>,
        additional_params: Option<HashMap<String, String>>,
        use_gitmoji: Option<bool>,
        instructions: Option<String>,
        token_limit: Option<usize>,
    ) -> Result<()> {
        if let Some(ref provider_name) = provider {
            // Validate provider
            let parsed: Provider = provider_name.parse().with_context(|| {
                format!(
                    "Unknown provider '{}'. Supported: {}",
                    provider_name,
                    Provider::all_names().join(", ")
                )
            })?;

            self.default_provider = parsed.name().to_string();

            // Ensure provider config exists
            if !self.providers.contains_key(parsed.name()) {
                self.providers.insert(
                    parsed.name().to_string(),
                    ProviderConfig::with_defaults(parsed),
                );
            }
        }

        let provider_config = self
            .providers
            .get_mut(&self.default_provider)
            .context("Could not get default provider config")?;

        if let Some(key) = api_key {
            provider_config.api_key = key;
        }
        if let Some(m) = model {
            provider_config.model = m;
        }
        if let Some(fm) = fast_model {
            provider_config.fast_model = Some(fm);
        }
        if let Some(params) = additional_params {
            provider_config.additional_params.extend(params);
        }
        if let Some(gitmoji) = use_gitmoji {
            self.use_gitmoji = gitmoji;
        }
        if let Some(instr) = instructions {
            self.instructions = instr;
        }
        if let Some(limit) = token_limit {
            provider_config.token_limit = Some(limit);
        }

        log_debug!("Configuration updated");
        Ok(())
    }

    /// Get the provider configuration for a specific provider
    #[must_use]
    pub fn get_provider_config(&self, provider: &str) -> Option<&ProviderConfig> {
        // Handle legacy/common aliases
        let name = if provider.eq_ignore_ascii_case("claude") {
            "anthropic"
        } else if provider.eq_ignore_ascii_case("gemini") {
            "google"
        } else {
            provider
        };

        self.providers
            .get(name)
            .or_else(|| self.providers.get(&name.to_lowercase()))
    }

    /// Get the current provider as `Provider` enum
    #[must_use]
    pub fn provider(&self) -> Option<Provider> {
        self.default_provider.parse().ok()
    }

    /// Validate that the current provider is properly configured
    ///
    /// # Errors
    ///
    /// Returns an error when the provider is invalid or no API key is configured.
    pub fn validate(&self) -> Result<()> {
        let provider: Provider = self
            .default_provider
            .parse()
            .with_context(|| format!("Invalid provider: {}", self.default_provider))?;

        let config = self
            .get_provider_config(provider.name())
            .ok_or_else(|| anyhow!("No configuration found for provider: {}", provider.name()))?;

        if !config.has_api_key() {
            // Check environment variable as fallback
            if std::env::var(provider.api_key_env()).is_err() {
                return Err(anyhow!(
                    "API key required for {}. Set {} or configure in ~/.config/git-iris/config.toml",
                    provider.name(),
                    provider.api_key_env()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
