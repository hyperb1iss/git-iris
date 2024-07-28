use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub api_key: String,
    #[serde(default)]
    pub use_gitmoji: bool,
    #[serde(default)]
    pub custom_instructions: String,
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
            .map(|path| path.join(".gitiris"))
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
        api_key: Option<String>,
        use_gitmoji: Option<bool>,
        custom_instructions: Option<String>,
    ) {
        if let Some(key) = api_key {
            self.api_key = key;
        }
        if let Some(gitmoji) = use_gitmoji {
            self.use_gitmoji = gitmoji;
        }
        if let Some(instructions) = custom_instructions {
            self.custom_instructions = instructions;
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_key: String::new(),
            use_gitmoji: false,
            custom_instructions: String::new(),
        }
    }
}
