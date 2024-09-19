use std::io;

use atlas_server::{cli::Commands, commands, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Configuration(command) => commands::configuration(command).await?,
        Commands::Dataset(command) => commands::dataset(command).await?,
        Commands::Run(command) => commands::run(command).await?,
        Commands::Server(config) => commands::server(config).await?,
        Commands::Worker(config) => commands::worker(config).await?,
    }

    Ok(())
}
