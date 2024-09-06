use std::io;

use atlas::{cli::Commands, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Configuration(command) => atlas::commands::configuration(command).await?,
        Commands::Dataset(command) => atlas::commands::dataset(command).await?,
        Commands::Run(command) => atlas::commands::run(command).await?,
        Commands::Server(config) => atlas::commands::server(config).await?,
        Commands::Worker(config) => atlas::commands::worker(config).await?,
    }

    Ok(())
}
