use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub api_key: String,
    // Add other configuration options as needed
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Config::get_config_path()?;
        let config_content = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }

    fn get_config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Unable to determine home directory"))?;
        Ok(home_dir.join(".gitllmconfig"))
    }
}