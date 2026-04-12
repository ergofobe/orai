mod attachment;
mod chat;
mod cli;
mod client;
mod markdown;
mod prompt;
mod stream;
mod tools;
mod tui;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    if std::env::var("OPENROUTER_API_KEY").is_err() {
        anyhow::bail!("OPENROUTER_API_KEY environment variable is required.\nSet it with: export OPENROUTER_API_KEY=your-key-here");
    }

    match &cli.command {
        cli::Commands::Prompt { .. } => prompt::run_prompt(&cli, &cli.command).await?,
        cli::Commands::Chat => chat::run_chat(&cli).await?,
        cli::Commands::Tui => tui::run_tui(&cli).await?,
    }

    Ok(())
}
