use git_llm::{cli, git, prompt, llm, config, file_analyzers};

use anyhow::Result;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Check environment and configuration
    if let Err(e) = Config::check_environment() {
        eprintln!("Error: {}", e);
        eprintln!("\nPlease ensure the following:");
        eprintln!("1. Git is installed and accessible from the command line.");
        eprintln!("2. You are running this command from within a Git repository.");
        eprintln!("3. You have created a .gitllmconfig file in your home directory with your OpenAI API key.");
        eprintln!("\nExample .gitllmconfig content:");
        eprintln!("api_key = \"your_openai_api_key_here\"");
        eprintln!("use_gitmoji = true  # Optional: set to false if you don't want to use gitmoji");
        return Ok(());
    }

    let config = Config::load()?;
    let args = cli::parse_args();

    match args.command {
        cli::Commands::Gen { verbose } => {
            let git_info = git::get_git_info()?;
            
            if git_info.staged_files.is_empty() {
                println!("No staged changes. Please stage your changes before generating a commit message.");
                println!("You can stage changes using 'git add <file>' or 'git add .'");
                return Ok(());
            }

            let prompt = prompt::create_prompt(&git_info, &config, verbose)?;
            let generated_message = llm::get_refined_message(&prompt, config.use_gitmoji, verbose).await?;
            
            println!("Generated commit message:\n{}", generated_message);
            
            if args.auto_commit {
                git::commit(&generated_message)?;
                println!("Changes committed successfully.");
            } else {
                println!("\nTo commit with this message, run:");
                println!("git commit -m \"{}\"", generated_message.replace("\"", "\\\""));
            }
        }
    }
    
    Ok(())
}