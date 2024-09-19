use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Format {
    /// htseq-count counts.
    HtseqCount,
    /// STAR counts.
    Star,
}

impl From<Format> for atlas_core::counts::reader::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::HtseqCount => Self::HtseqCount,
            Format::Star => Self::Star,
        }
    }
}
