use crate::llm_provider::LLMProviderConfig;
use crate::provider_registry::ProviderRegistry;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub default_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub use_gitmoji: bool,
    #[serde(default)]
    pub custom_instructions: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub additional_params: HashMap<String, String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Config::get_config_path()?;
        if !config_path.exists() {
            return Ok(Config::default());
        }
        let config_content = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Config::get_config_path()?;
        let config_content = toml::to_string(self)?;
        fs::write(config_path, config_content)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        dirs::home_dir()
            .ok_or_else(|| anyhow!("Unable to determine home directory"))
            .map(|path| path.join(".git-iris"))
    }

    pub fn check_environment() -> Result<()> {
        // Check if git is installed
        if !Command::new("git").arg("--version").output().is_ok() {
            return Err(anyhow!(
                "Git is not installed or not in PATH. Please install Git and try again."
            ));
        }

        // Check if we're in a git repository
        if !Command::new("git")
            .args(&["rev-parse", "--is-inside-work-tree"])
            .output()?
            .status
            .success()
        {
            return Err(anyhow!(
                "Not in a Git repository. Please run this command from within a Git repository."
            ));
        }

        Ok(())
    }

    pub fn update(
        &mut self,
        provider: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
        additional_params: Option<HashMap<String, String>>,
        use_gitmoji: Option<bool>,
        custom_instructions: Option<String>,
    ) {
        if let Some(provider) = provider {
            self.default_provider = provider.clone();
            if !self.providers.contains_key(&provider) {
                self.providers
                    .insert(provider.clone(), ProviderConfig::default_for(&provider));
            }
        }

        let provider_config = self.providers.get_mut(&self.default_provider).unwrap();

        if let Some(key) = api_key {
            provider_config.api_key = key;
        }
        if let Some(model) = model {
            provider_config.model = model;
        }
        if let Some(params) = additional_params {
            provider_config.additional_params.extend(params);
        }
        if let Some(gitmoji) = use_gitmoji {
            self.use_gitmoji = gitmoji;
        }
        if let Some(instructions) = custom_instructions {
            self.custom_instructions = instructions;
        }
    }

    pub fn get_provider_config(&self, provider: &str) -> Option<&ProviderConfig> {
        self.providers.get(provider)
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("openai".to_string(), ProviderConfig::default_for("openai"));
        providers.insert("claude".to_string(), ProviderConfig::default_for("claude"));

        Config {
            default_provider: "openai".to_string(),
            providers,
            use_gitmoji: false,
            custom_instructions: String::new(),
        }
    }
}

impl ProviderConfig {
    pub fn default_for(provider: &str) -> Self {
        let default_model = ProviderRegistry::default()
            .get_default_model(provider)
            .unwrap_or_else(|| {
                panic!(
                    "Default model for provider '{}' not found in registry",
                    provider
                );
            });

        ProviderConfig {
            api_key: String::new(),
            model: default_model.to_string(),
            additional_params: HashMap::new(),
        }
    }

    pub fn to_llm_provider_config(&self) -> LLMProviderConfig {
        LLMProviderConfig {
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            additional_params: self.additional_params.clone(),
        }
    }
}
