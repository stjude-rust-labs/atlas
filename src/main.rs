use atlas::{cli::Commands, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Import(_) => unimplemented!(),
        Commands::Run(config) => atlas::commands::run(config).await?,
    }

    Ok(())
}
