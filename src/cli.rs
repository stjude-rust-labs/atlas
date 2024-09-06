pub mod configuration;
pub mod dataset;
pub mod run;
mod server;
mod worker;

use clap::{Parser, Subcommand};

pub use self::{server::ServerConfig, worker::WorkerConfig};

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage configurations
    #[clap(subcommand)]
    Configuration(configuration::Command),
    /// Manage datasets
    #[clap(subcommand)]
    Dataset(dataset::Command),
    /// Manage runs
    #[clap(subcommand)]
    Run(run::Command),
    /// Starts an atlas server and blocks indefinitely
    Server(ServerConfig),
    /// Starts an atlas worker.
    Worker(WorkerConfig),
}
