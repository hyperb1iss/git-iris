use anyhow::Result;
use git_iris::cli;

/// Main entry point for the application
#[tokio::main]
async fn main() -> Result<()> {
    git_iris::logger::init().expect("Failed to initialize logger");
    match cli::main().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
