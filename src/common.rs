use crate::config::Config;
use crate::instruction_presets::get_instruction_preset_library;
use crate::llm_providers::LLMProviderType;
use anyhow::Result;
use clap::Args;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DetailLevel {
    Minimal,
    Standard,
    Detailed,
}

impl FromStr for DetailLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(DetailLevel::Minimal),
            "standard" => Ok(DetailLevel::Standard),
            "detailed" => Ok(DetailLevel::Detailed),
            _ => Err(anyhow::anyhow!("Invalid detail level: {}", s)),
        }
    }
}

impl DetailLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DetailLevel::Minimal => "minimal",
            DetailLevel::Standard => "standard",
            DetailLevel::Detailed => "detailed",
        }
    }
}

#[derive(Args, Clone, Default, Debug)]
pub struct CommonParams {
    /// Override default LLM provider
    #[arg(long, help = "Override default LLM provider", value_parser = available_providers_parser)]
    pub provider: Option<String>,

    /// Custom instructions for this operation
    #[arg(short, long, help = "Custom instructions for this operation")]
    pub instructions: Option<String>,

    /// Select an instruction preset
    #[arg(long, help = "Select an instruction preset")]
    pub preset: Option<String>,

    /// Enable or disable Gitmoji
    #[arg(long, help = "Enable or disable Gitmoji")]
    pub gitmoji: Option<bool>,

    /// Set the detail level
    #[arg(
        long,
        help = "Set the detail level (minimal, standard, detailed)",
        default_value = "standard"
    )]
    pub detail_level: String,
}

impl CommonParams {
    pub fn apply_to_config(&self, config: &mut Config) -> Result<()> {
        if let Some(provider) = &self.provider {
            config.default_provider = LLMProviderType::from_str(provider)?.to_string();
        }
        if let Some(instructions) = &self.instructions {
            config.set_temp_instructions(Some(instructions.clone()));
        }
        if let Some(preset) = &self.preset {
            config.set_temp_preset(Some(preset.clone()));
        }
        if let Some(use_gitmoji) = self.gitmoji {
            config.use_gitmoji = use_gitmoji;
        }
        Ok(())
    }
}

/// Validate provider input against available providers
pub fn available_providers_parser(s: &str) -> Result<String, String> {
    let available_providers = crate::llm::get_available_provider_names();
    if available_providers.contains(&s.to_lowercase()) && s.to_lowercase() != "test" {
        Ok(s.to_lowercase())
    } else {
        Err(format!(
            "Invalid provider. Available providers are: {}",
            available_providers.join(", ")
        ))
    }
}

pub fn get_combined_instructions(config: &Config) -> String {
    let mut prompt = String::from("\n\n");

    let preset_library = get_instruction_preset_library();
    if let Some(preset_instructions) = preset_library.get_preset(config.instruction_preset.as_str())
    {
        prompt.push_str(&format!(
            "\n\nUse this style for the commit message:\n{}\n\n",
            preset_instructions.instructions
        ));
    }

    if !config.instructions.is_empty() {
        prompt.push_str(&format!(
            "\n\nAdditional instructions for the commit message:\n{}\n\n",
            config.instructions
        ));
    }

    prompt
}
