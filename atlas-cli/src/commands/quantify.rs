mod count;
mod match_intervals;
mod specification;

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
use indexmap::IndexSet;
use noodles::{bam, core::Position, gff::record::Strand, sam};
use thiserror::Error;
use tracing::info;

use self::{count::count_single_records, specification::LibraryLayout};
use crate::cli::quantify;

type Features = HashMap<String, Vec<Feature>>;
type Entry<'f> = (&'f str, Strand);
type IntervalTrees<'f> = Vec<IntervalTree<Position, Entry<'f>>>;

pub fn quantify(args: quantify::Args) -> Result<(), QuantifyError> {
    let annotations_src = &args.annotations;
    let feature_type = &args.feature_type;
    let feature_id = &args.feature_id;

    info!(
        src = ?annotations_src,
        feature_type, feature_id, "reading features"
    );

    let (reference_sequence_names, features) =
        read_features(annotations_src, feature_type, feature_id)?;

    info!(feature_count = features.len(), "read features");

    let src = &args.src;

    info!(src = ?src, "reading alignment header");

    let mut reader = bam::io::reader::Builder.build_from_path(src)?;
    let header = reader.read_header()?;

    info!(
        reference_sequence_count = header.reference_sequences().len(),
        "read alignment header"
    );

    info!("building interval trees");

    let interval_trees = build_interval_trees(&header, &reference_sequence_names, &features);

    info!(
        interval_tree_count = interval_trees.len(),
        "built interval trees"
    );

    info!("detecting library type");

    let (library_layout, strand_specification) =
        specification::detect(&mut reader, &interval_trees)?;

    info!(
        ?library_layout,
        ?strand_specification,
        "detected library layout"
    );

    let mut reader = bam::io::reader::Builder.build_from_path(src)?;
    reader.read_header()?;

    let _ctx = match library_layout {
        LibraryLayout::Single => count_single_records(&interval_trees, reader)?,
        LibraryLayout::Multiple => todo!(),
    };

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
) -> Result<(IndexSet<String>, Features), QuantifyError>
where
    P: AsRef<Path>,
{
    use atlas_core::features::read_features;

    let mut reader = File::open(src).map(BufReader::new)?;
    let (reference_sequence_names, features) =
        read_features(&mut reader, feature_type, feature_id)?;
    Ok((reference_sequence_names, features))
}

fn build_interval_trees<'f>(
    header: &sam::Header,
    reference_sequence_names: &IndexSet<String>,
    features: &'f Features,
) -> IntervalTrees<'f> {
    let reference_sequences = header.reference_sequences();

    let mut interval_trees = Vec::new();
    interval_trees.resize_with(reference_sequences.len(), IntervalTree::default);

    for (name, segments) in features {
        for feature in segments {
            let Feature {
                reference_sequence_id,
                start,
                end,
                strand,
            } = *feature;

            let reference_sequence_name = reference_sequence_names
                .get_index(reference_sequence_id)
                .unwrap();

            let Some(i) = reference_sequences.get_index_of(reference_sequence_name.as_bytes())
            else {
                continue;
            };

            // SAFETY: `interval_trees.len() == reference_sequences.len()`.
            let tree = &mut interval_trees[i];

            tree.insert(start..=end, (name.as_str(), strand));
        }
    }

    interval_trees
}
