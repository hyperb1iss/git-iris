use anyhow::Result;
use git_iris::cli;

#[tokio::main]
async fn main() -> Result<()> {
    cli::main().await
}