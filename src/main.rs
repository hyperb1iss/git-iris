use anyhow::Result;
use git_iris::cli;

/// Main entry point for the application
#[tokio::main]
async fn main() -> Result<()> {
    cli::main().await
}
