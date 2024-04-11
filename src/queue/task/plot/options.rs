use serde::{Deserialize, Serialize};

/// Barnes-Hut t-SNE options.
#[derive(Deserialize, Serialize)]
pub struct Options {
    /// Perplexity of the conditional distribution.
    pub perplexity: f64,
    /// Barnes-Hut theta.
    pub theta: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            perplexity: 30.0,
            theta: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let options = Options::default();
        assert_eq!(options.perplexity, 30.0);
        assert_eq!(options.theta, 0.5);
    }
}
