use super::PERPLEXITY;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("insufficient number of samples: got {0}, expected > 3 * {PERPLEXITY}")]
    InsufficientSampleCount(usize),
}
