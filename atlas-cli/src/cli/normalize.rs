use std::path::PathBuf;

use atlas_core as core;
use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Method {
    /// Fragments per kilobase per million (FPKM) mapped reads.
    Fpkm,
    /// Median of ratios.
    MedianOfRatios,
    /// Trimmed mean of M-values (TMM).
    Tmm,
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

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    HtseqCount,
    Star,
}

impl From<Format> for core::counts::reader::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::HtseqCount => Self::HtseqCount,
            Format::Star => Self::Star,
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
    ///
    /// This is only used if the input format is STAR.
    #[arg(long, value_enum, default_value_t = StrandSpecification::Forward)]
    pub strand_specification: StrandSpecification,

    /// The input format.
    ///
    /// By default, the format is autodetected.
    #[arg(long, value_enum)]
    pub format: Option<Format>,

    /// Input sources (htseq-count or STAR).
    #[arg(required = true)]
    pub srcs: Vec<PathBuf>,
}
