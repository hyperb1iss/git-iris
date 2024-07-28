use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize)]
pub struct Config {
    pub api_key: String,
    pub use_gitmoji: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Config::get_config_path()?;
        if !config_path.exists() {
            return Err(anyhow!("Configuration file not found. Please create a .gitiris file in your home directory."));
        }
        let config_content = fs::read_to_string(config_path)?;
        let mut config: Config = toml::from_str(&config_content)?;

        // Set default value for use_gitmoji if not specified
        if !config_content.contains("use_gitmoji") {
            config.use_gitmoji = false;
        }

        Ok(config)
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

        // Load config (this will check for the .gitiris file)
        let config = Self::load()?;

        // Check if API key is set
        if config.api_key.is_empty() {
            return Err(anyhow!("API key is not set in .gitiris. Please add your OpenAI API key to the configuration file."));
        }

        Ok(())
    }
}
