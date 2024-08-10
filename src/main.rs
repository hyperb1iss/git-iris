use anyhow::Result;
use git_iris::cli;

/// Main entry point for the application
#[tokio::main]
async fn main() -> Result<()> {
    git_iris::logger::init().expect("Failed to initialize logger");
    cli::main().await
}
