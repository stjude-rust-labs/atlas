pub mod configuration;

use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};

use crate::{counts, store::StrandSpecification};

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
    /// Imports a sample into the database
    Import(ImportConfig),
    /// Starts an atlas server and blocks indefinitely
    Server(ServerConfig),
    /// Starts an atlas worker.
    Worker(WorkerConfig),
}

#[derive(Debug, Parser)]
pub struct ImportConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The configuration ID.
    #[clap(long)]
    pub configuration_id: i32,

    /// The strand specification used when counting features.
    #[clap(value_enum, long)]
    pub strand_specification: StrandSpecification,

    /// The sample name delimiter.
    ///
    /// This is used to split the source input's filename to use as the sample's name.
    #[clap(long, default_value = ".")]
    pub sample_name_delimiter: String,

    /// The technique/process used to sequence the given sample, e.g., "RNA-Seq", etc.
    #[clap(long)]
    pub data_type: String,

    /// The input format.
    ///
    /// By default, the format is autodetected.
    #[clap(long)]
    pub format: Option<counts::Format>,

    /// Set whether the input is a sample sheet.
    ///
    /// This is used to bulk import many samples with the same annotation and
    /// configuration. The input format is tab-separated plain text (no header)
    /// with two columns: sample name and source path.
    #[clap(long)]
    pub sample_sheet: bool,

    /// The input sources.
    ///
    /// The inputs can be feature count outputs from either htseq-count or STAR.
    /// If the `--sample-sheet` flag is set, the inputs must be tab-separated
    /// plain text files.
    #[clap(required = true)]
    pub srcs: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ServerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The socket address the server binds to.
    #[clap(long, env = "BIND_ADDRESS", default_value = "127.0.0.1:3000")]
    pub bind: SocketAddr,
}

#[derive(Debug, Parser)]
pub struct WorkerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,
}
