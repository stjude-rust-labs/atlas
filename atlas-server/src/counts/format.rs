use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Format {
    /// htseq-count counts.
    HtseqCount,
    /// STAR counts.
    Star,
}
