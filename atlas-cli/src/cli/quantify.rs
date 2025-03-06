use std::{num::NonZero, path::PathBuf};

use clap::{Parser, ValueEnum};
use noodles::sam::alignment::record::MappingQuality;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum StrandSpecificationOption {
    None,
    Forward,
    Reverse,
    Auto,
}

#[derive(Parser)]
pub struct Args {
    /// Feature type.
    #[arg(long, default_value = "exon")]
    pub feature_type: String,

    /// Feature ID.
    #[arg(long, default_value = "gene_id")]
    pub feature_id: String,

    /// Input annotations file (GFF3).
    ///
    /// This can be uncompressed or (b)gzip-compressed.
    #[arg(long)]
    pub annotations: PathBuf,

    /// Record mapping quality threshold.
    #[arg(long, value_parser = parse_mapping_quality, default_value = "10")]
    pub min_mapping_quality: MappingQuality,

    /// Strand specification.
    #[arg(long, value_enum, default_value_t = StrandSpecificationOption::Auto)]
    pub strand_specification: StrandSpecificationOption,

    /// Output destination.
    ///
    /// If not set, output is written to stdout.
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// The number of workers to spawn.
    ///
    /// By default, this (usually) uses the number of available CPUs.
    pub worker_count: Option<NonZero<usize>>,

    /// Source input (BAM).
    pub src: PathBuf,
}

fn parse_mapping_quality(s: &str) -> Result<MappingQuality, &'static str> {
    s.parse::<u8>()
        .map_err(|_| "invalid input")
        .and_then(|n| MappingQuality::new(n).ok_or("missing mapping quality"))
}
