#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("perplexity too large: perplexity ({perplexity}) must be < ({sample_count} - 1) / 3")]
    PerplexityTooLarge {
        sample_count: usize,
        perplexity: f64,
    },
}
