use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Import a configuration
    Import(ImportConfig),
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

    /// The input source in GFF3.
    pub src: PathBuf,
}
