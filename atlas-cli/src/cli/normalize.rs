use std::path::PathBuf;

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

    /// Input source (htseq-count or STAR).
    pub src: PathBuf,
}
