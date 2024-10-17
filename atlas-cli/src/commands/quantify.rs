use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use atlas_core::features::{Feature, ReadFeaturesError};
use thiserror::Error;
use tracing::info;

use crate::cli::quantify;

pub fn quantify(args: quantify::Args) -> Result<(), QuantifyError> {
    let annotations_src = &args.annotations;
    let feature_type = &args.feature_type;
    let feature_id = &args.feature_id;

    info!(
        src = ?annotations_src,
        feature_type, feature_id, "reading features"
    );

    let features = read_features(annotations_src, feature_type, feature_id)?;

    info!(feature_count = features.len(), "read features");

    todo!()
}

#[derive(Debug, Error)]
pub enum QuantifyError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("invalid features")]
    InvalidFeatures(#[from] ReadFeaturesError),
}

fn read_features<P>(
    src: P,
    feature_type: &str,
    feature_id: &str,
) -> Result<HashMap<String, Vec<Feature>>, QuantifyError>
where
    P: AsRef<Path>,
{
    use atlas_core::features::read_features;

    let mut reader = File::open(src).map(BufReader::new)?;
    let features = read_features(&mut reader, feature_type, feature_id)?;
    Ok(features)
}
