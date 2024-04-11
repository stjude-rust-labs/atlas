use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use thiserror::Error;
use utoipa::ToSchema;

use crate::queue::task::plot;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ValidateError {
    #[error("length mismatch: expected {expected}, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },
    #[error("invalid name: {0}")]
    InvalidName(String),
}

pub(super) fn validate_run(
    feature_names: &HashSet<String>,
    run: &HashMap<String, i32>,
) -> Result<(), ValidateError> {
    if run.len() != feature_names.len() {
        return Err(ValidateError::LengthMismatch {
            expected: feature_names.len(),
            actual: run.len(),
        });
    }

    for name in run.keys() {
        if !feature_names.contains(name) {
            return Err(ValidateError::InvalidName(name.into()));
        }
    }

    Ok(())
}

#[derive(Deserialize, ToSchema)]
pub struct Options {
    perplexity: Option<f64>,
    theta: Option<f64>,
}

pub(super) fn merge_options(options: &mut plot::Options, arguments: &Options) {
    if let Some(perplexity) = arguments.perplexity {
        options.perplexity = perplexity;
    }

    if let Some(theta) = arguments.theta {
        options.theta = theta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_run() {
        let feature_names = [String::from("f0"), String::from("f1")]
            .into_iter()
            .collect();

        let run = [(String::from("f0"), 0), (String::from("f1"), 0)]
            .into_iter()
            .collect();
        assert!(validate_run(&feature_names, &run).is_ok());

        let run = [(String::from("f0"), 0)].into_iter().collect();
        assert_eq!(
            validate_run(&feature_names, &run),
            Err(ValidateError::LengthMismatch {
                expected: 2,
                actual: 1,
            })
        );

        let run = [
            (String::from("f0"), 0),
            (String::from("f1"), 0),
            (String::from("f2"), 0),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            validate_run(&feature_names, &run),
            Err(ValidateError::LengthMismatch {
                expected: 2,
                actual: 3,
            })
        );

        let run = [(String::from("f2"), 0), (String::from("f0"), 0)]
            .into_iter()
            .collect();
        assert_eq!(
            validate_run(&feature_names, &run),
            Err(ValidateError::InvalidName(String::from("f2")))
        );
    }

    #[test]
    fn test_merge_options() {
        let defualt_options = plot::Options::default();

        let mut options = plot::Options::default();
        let arguments = Options {
            perplexity: None,
            theta: None,
        };
        merge_options(&mut options, &arguments);
        assert_eq!(options.perplexity, defualt_options.perplexity);
        assert_eq!(options.theta, defualt_options.theta);

        let mut options = plot::Options::default();
        let arguments = Options {
            perplexity: Some(10.0),
            theta: Some(0.3),
        };
        merge_options(&mut options, &arguments);
        assert_eq!(options.perplexity, 10.0);
        assert_eq!(options.theta, 0.3);
    }
}
