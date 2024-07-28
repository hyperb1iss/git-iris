mod cli;
mod git;
mod prompt;
mod llm;
mod config;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::parse_args();

    match args.command {
        cli::Commands::Gen => {
            let git_info = git::get_git_info()?;
            
            if git_info.staged_files.is_empty() {
                println!("No staged changes. Please stage your changes before generating a commit message.");
                return Ok(());
            }

            let prompt = prompt::create_prompt(&git_info)?;
            let generated_message = llm::get_refined_message(&prompt).await?;
            
            println!("Generated commit message:\n{}", generated_message);
            
            if args.auto_commit {
                git::commit(&generated_message)?;
                println!("Changes committed successfully.");
            } else {
                println!("To commit with this message, run:");
                println!("git commit -m \"{}\"", generated_message.replace("\"", "\\\""));
            }
        }
    }
    
    Ok(())
}