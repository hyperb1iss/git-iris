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
        cli::Commands::Gen { message } => {
            let git_info = git::get_git_info()?;
            let prompt = prompt::create_prompt(&message, &git_info)?;
            let refined_message = llm::get_refined_message(&prompt).await?;
            
            println!("Refined commit message: {}", refined_message);
            
            if args.auto_commit {
                git::commit(&refined_message)?;
                println!("Changes committed successfully.");
            }
        }
    }
    
    Ok(())
}