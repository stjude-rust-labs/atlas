use atlas::{cli::Commands, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Import(config) => atlas::commands::import(config).await?,
        Commands::Server(config) => atlas::commands::server(config).await?,
        Commands::Worker(config) => atlas::commands::worker(config).await?,
    }

    Ok(())
}
