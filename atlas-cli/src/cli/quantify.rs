use std::path::PathBuf;

use clap::Parser;
use noodles::sam::alignment::record::MappingQuality;

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

    /// Record mapping quality threshold.
    #[arg(long, value_parser = parse_mapping_quality, default_value = "10")]
    pub min_mapping_quality: MappingQuality,

    /// Source input (BAM).
    pub src: PathBuf,
}

fn parse_mapping_quality(s: &str) -> Result<MappingQuality, &'static str> {
    s.parse::<u8>()
        .map_err(|_| "invalid input")
        .and_then(|n| MappingQuality::new(n).ok_or("missing mapping quality"))
}
