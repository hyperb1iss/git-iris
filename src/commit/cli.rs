use anyhow::Result;
use crate::common::CommonParams;
use crate::config::Config;
use crate::context::format_commit_message;
use crate::llm_providers::LLMProviderType;
use crate::messages;
use crate::ui;
use crate::tui::run_tui_commit;
use super::service::IrisCommitService;
use std::str::FromStr;
use std::sync::Arc;

pub async fn handle_gen_command(
    common: CommonParams,
    auto_commit: bool,
    use_gitmoji: bool,
    print: bool,
) -> Result<()> {
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;
    let current_dir = std::env::current_dir()?;

    let provider_type = LLMProviderType::from_str(&config.default_provider)?;

    let service = Arc::new(IrisCommitService::new(
        config.clone(),
        current_dir.clone(),
        provider_type.clone(),
        use_gitmoji && config.use_gitmoji,
    ));

    // Check environment prerequisites
    if let Err(e) = service.check_environment() {
        ui::print_error(&format!("Error: {}", e));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info("2. You are running this command from within a Git repository.");
        ui::print_info("3. You have set up your configuration using 'git-iris config'.");
        return Err(e);
    }

    let git_info = service.get_git_info().await?;

    if git_info.staged_files.is_empty() {
        ui::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());
    let preset_str = common.preset.as_deref().unwrap_or("");

    // Create and start the spinner
    let spinner = ui::create_spinner("");
    let random_message = messages::get_random_message();
    spinner.set_message(random_message.text);

    // Generate an initial message
    let initial_message = service
        .generate_message(preset_str, &effective_instructions)
        .await?;

    // Stop the spinner
    spinner.finish_and_clear();

    if print {
        println!("{}", format_commit_message(&initial_message));
        return Ok(());
    }

    if auto_commit {
        service.perform_commit(&format_commit_message(&initial_message))?;
        println!(
            "🌟 Commit created with message: {}",
            format_commit_message(&initial_message)
        );
        return Ok(());
    }

    run_tui_commit(
        vec![initial_message],
        effective_instructions,
        String::from(preset_str),
        git_info.user_name,
        git_info.user_email,
        service,
        config.use_gitmoji,
    )
    .await?;

    Ok(())
}