use std::path::PathBuf;

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

    /// The name of the annotations, e.g., "GENCODE 40", etc.
    #[clap(long)]
    pub annotations_name: String,

    /// The assembly name of the genome used to create the annotations, e.g., "GRCh37",
    /// "GRCh38.p13", etc.
    #[clap(long)]
    pub annotations_genome_build: String,

    /// The type of feature used in the annotations, e.g., "exon", "gene", etc.
    #[clap(long)]
    pub feature_type: String,

    /// The display name of the feature, e.g., "gene_name", "gene_id", etc.
    #[clap(long)]
    pub feature_name: String,

    /// The strand specification used when counting features.
    #[clap(value_enum, long)]
    pub strand_specification: StrandSpecification,

    /// The sample name delimiter.
    ///
    /// This is used to split the source input's filename to use as the sample's name.
    #[clap(long, default_value = ".")]
    pub sample_name_delimiter: String,

    /// The technique process used to sequence the given sample, e.g., "RNA-Seq", etc.
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

    /// The port for the server to use.
    #[clap(long, env, default_value_t = 3000)]
    pub port: u16,
}

#[derive(Debug, Parser)]
pub struct WorkerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,
}
