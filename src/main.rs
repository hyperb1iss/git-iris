use anyhow::Result;
use git_iris::{cli, config, git, interactive, llm};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::parse_args();
    let mut config = config::Config::load()?;

    match args.command {
        cli::Commands::Gen { verbose, gitmoji } => {
            if let Err(e) = config::Config::check_environment() {
                cli::print_error(&format!("Error: {}", e));
                cli::print_info("\nPlease ensure the following:");
                cli::print_info("1. Git is installed and accessible from the command line.");
                cli::print_info("2. You are running this command from within a Git repository.");
                cli::print_info("3. You have set up your configuration using 'git-iris config'.");
                return Ok(());
            }

            if config.api_key.is_empty() {
                cli::print_error("API key is not set. Please run 'git-iris config --api-key YOUR_API_KEY' to set it.");
                return Ok(());
            }

            let current_dir = std::env::current_dir()?;
            let git_info = git::get_git_info(current_dir.as_path())?;

            if git_info.staged_files.is_empty() {
                cli::print_warning("No staged changes. Please stage your changes before generating a commit message.");
                cli::print_info("You can stage changes using 'git add <file>' or 'git add .'");
                return Ok(());
            }

            let use_gitmoji = gitmoji.unwrap_or(config.use_gitmoji);

            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                    .template("{spinner} Generating initial commit message...")?,
            );
            spinner.enable_steady_tick(Duration::from_millis(100));

            let initial_message = llm::get_refined_message(&git_info, &config, use_gitmoji, verbose).await?;

            spinner.finish_and_clear();

            let mut interactive_commit = interactive::InteractiveCommit::new(initial_message);

            let commit_performed = interactive_commit
                .run(|| async {
                    let git_info = git::get_git_info(current_dir.as_path())?;
                    llm::get_refined_message(&git_info, &config, use_gitmoji, verbose).await
                })
                .await?;

            if commit_performed {
                cli::print_success("Commit successfully created and applied.");
            } else {
                cli::print_info("Commit process cancelled.");
            }
        }
        cli::Commands::Config {
            api_key,
            llm_provider,
            gitmoji,
            custom_instructions,
        } => {
            config.update(api_key, llm_provider.map(|l| l.to_string()), gitmoji, custom_instructions);
            config.save()?;
            cli::print_success("Configuration updated successfully.");
            cli::print_info(&format!(
                "Current configuration:\nAPI Key: {}\nLLM Provider: {}\nUse Gitmoji: {}\nCustom Instructions: {}",
                if config.api_key.is_empty() {
                    "Not set"
                } else {
                    "Set"
                },
                config.llm_provider,
                config.use_gitmoji,
                if config.custom_instructions.is_empty() {
                    "None".to_string()
                } else {
                    config.custom_instructions.replace('\n', ", ")
                }
            ));
        }
    }

    Ok(())
}