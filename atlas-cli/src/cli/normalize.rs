use std::path::PathBuf;

use atlas_core as core;
use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, ValueEnum)]
pub enum Method {
    /// Fragments per kilobase per million (FPKM) mapped reads.
    Fpkm,
    /// Median of ratios.
    MedianOfRatios,
    /// Transcripts per million (TPM) mapped reads
    Tpm,
}

#[derive(Clone, Copy, Default, ValueEnum)]
pub enum StrandSpecification {
    None,
    #[default]
    Forward,
    Reverse,
}

impl From<StrandSpecification> for core::StrandSpecification {
    fn from(strand_specification: StrandSpecification) -> Self {
        match strand_specification {
            StrandSpecification::None => Self::None,
            StrandSpecification::Forward => Self::Forward,
            StrandSpecification::Reverse => Self::Reverse,
        }
    }
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
    #[arg(long)]
    pub annotations: PathBuf,

    /// Normalization method.
    #[arg(long, value_enum, default_value_t = Method::Tpm)]
    pub method: Method,

    /// Strand specification.
    #[arg(long, value_enum, default_value_t = StrandSpecification::Forward)]
    pub strand_specification: StrandSpecification,

    /// Input sources (htseq-count or STAR).
    #[arg(required = true)]
    pub srcs: Vec<PathBuf>,
}
