use clap::{Parser, Subcommand};
use colored::*;
use std::collections::HashMap;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about = "AI-assisted Git commit message generator", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        short,
        long,
        global = true,
        help = "Automatically commit with the generated message"
    )]
    pub auto_commit: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(
        about = "Generate a commit message using AI",
        after_help = "Default LLM provider: openai\nAvailable providers: claude, openai"
    )]
    Gen {
        #[arg(short, long, help = "Enable verbose mode")]
        verbose: bool,

        #[arg(short, long, help = "Override use_gitmoji setting")]
        gitmoji: Option<bool>,

        #[arg(long, help = "Override default LLM provider")]
        provider: Option<String>,
    },
    #[command(about = "Configure the AI-assisted Git commit message generator")]
    Config {
        #[arg(long, help = "Set default LLM provider")]
        provider: Option<String>,

        #[arg(long, help = "Set API key for the specified provider")]
        api_key: Option<String>,

        #[arg(long, help = "Set model for the specified provider")]
        model: Option<String>,

        #[arg(
            long,
            help = "Set additional parameters for the specified provider (key=value)"
        )]
        param: Option<Vec<String>>,

        #[arg(short, long, help = "Set use_gitmoji preference")]
        gitmoji: Option<bool>,

        #[arg(
            short,
            long,
            help = "Set custom instructions (separate multiple instructions with newlines)"
        )]
        custom_instructions: Option<String>,
    },
}

pub fn parse_args() -> Cli {
    let cli = Cli::parse();
    if let Commands::Gen { .. } = cli.command {
        if std::env::args().any(|arg| arg == "--help") {
            print_dynamic_help();
        }
    }
    cli
}

pub fn print_success(message: &str) {
    println!("{}", message.green());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}

pub fn print_info(message: &str) {
    println!("{}", message.blue());
}

pub fn print_warning(message: &str) {
    println!("{}", message.yellow());
}

pub fn parse_additional_params(params: &[String]) -> HashMap<String, String> {
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

pub async fn handle_command(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Gen {
            verbose,
            gitmoji,
            provider,
        } => {
            let config = crate::config::Config::load()?;
            let provider_name = provider.unwrap_or(config.default_provider.clone());
            let provider_config = config.get_provider_config(&provider_name).ok_or_else(|| {
                anyhow::anyhow!("Provider '{}' not found in configuration", provider_name)
            })?;

            let provider_arc = crate::provider_registry::ProviderRegistry::default()
                .create_provider(&provider_name, provider_config.clone())
                .unwrap_or_else(|e| {
                    panic!("Failed to create provider {}: {}", provider_name, e);
                });

            // Use the provider to generate a commit message
            let system_prompt = crate::prompt::create_system_prompt(
                gitmoji.unwrap_or(config.use_gitmoji),
                &config.custom_instructions,
            );
            let git_info = crate::git::get_git_info(Path::new("."))?;
            let user_prompt = crate::prompt::create_user_prompt(&git_info, verbose)?;

            if verbose {
                println!("Using LLM provider: {}", provider_arc.provider_name());
                println!("System prompt:\n{}", system_prompt);
                println!("User prompt:\n{}", user_prompt);
            }

            let refined_message = provider_arc
                .generate_message(&system_prompt, &user_prompt)
                .await?;

            if verbose {
                println!("Generated message:\n{}", refined_message);
            }

            if cli.auto_commit {
                crate::git::commit(Path::new("."), &refined_message)?;
                print_success("Commit created successfully.");
            } else {
                print_success("Commit message generated successfully.");
            }
        }
        Commands::Config {
            provider,
            api_key,
            model,
            param,
            gitmoji,
            custom_instructions,
        } => {
            let mut config = crate::config::Config::load()?;

            if let Some(provider) = provider {
                config.update(Some(provider), None, None, None, None, None);
            }

            if let Some(api_key) = api_key {
                config.update(None, Some(api_key), None, None, None, None);
            }

            if let Some(model) = model {
                config.update(None, None, Some(model), None, None, None);
            }

            if let Some(params) = param {
                let additional_params = parse_additional_params(&params);
                config.update(None, None, None, Some(additional_params), None, None);
            }

            if let Some(gitmoji) = gitmoji {
                config.update(None, None, None, None, Some(gitmoji), None);
            }

            if let Some(instructions) = custom_instructions {
                config.update(None, None, None, None, None, Some(instructions));
            }

            config.save()?;
            print_success("Configuration updated successfully.");
        }
    }

    Ok(())
}

pub fn list_providers() -> Vec<String> {
    // Query the provider registry to get the list of available providers
    crate::provider_registry::ProviderRegistry::default().list_providers()
}

pub fn print_dynamic_help() {
    let providers = list_providers();
    let provider_list = providers.join(", ");
    println!("Available providers: {}", provider_list);
}
