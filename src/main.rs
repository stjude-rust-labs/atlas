use atlas::{cli::Commands, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Import(config) => atlas::commands::import(config).await?,
        Commands::Run(config) => atlas::commands::run(config).await?,
    }

    Ok(())
}
