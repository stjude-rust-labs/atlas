use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use atlas_core::{
    collections::IntervalTree,
    features::{Feature, ReadFeaturesError},
};
use noodles::core::Position;
use thiserror::Error;
use tracing::info;

use crate::cli::quantify;

type Features = HashMap<String, Vec<Feature>>;
type IntervalTrees<'f> = HashMap<String, IntervalTree<Position, &'f str>>;

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
    info!("building interval trees");

    let interval_trees = build_interval_trees(&features);

    info!(
        interval_tree_count = interval_trees.len(),
        "built interval trees"
    );

    todo!()
}

#[derive(Debug, Error)]
pub enum QuantifyError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("invalid features")]
    InvalidFeatures(#[from] ReadFeaturesError),
}

fn read_features<P>(src: P, feature_type: &str, feature_id: &str) -> Result<Features, QuantifyError>
where
    P: AsRef<Path>,
{
    use atlas_core::features::read_features;

    let mut reader = File::open(src).map(BufReader::new)?;
    let features = read_features(&mut reader, feature_type, feature_id)?;
    Ok(features)
}

fn build_interval_trees(features: &Features) -> IntervalTrees<'_> {
    let mut interval_trees = IntervalTrees::default();

    for (name, segments) in features {
        for feature in segments {
            let reference_sequence_name = &feature.reference_sequence_name;

            let tree = if let Some(tree) = interval_trees.get_mut(reference_sequence_name) {
                tree
            } else {
                interval_trees
                    .entry(reference_sequence_name.into())
                    .or_default()
            };

            let start = feature.start;
            let end = feature.end;

            tree.insert(start..=end, name)
        }
    }

    interval_trees
}
