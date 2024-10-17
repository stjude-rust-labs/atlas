use std::path::PathBuf;

use clap::Parser;

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
}
