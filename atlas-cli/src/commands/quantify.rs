mod count;
mod filter;
mod match_intervals;
mod segmented_reads;
mod specification;

use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
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

use self::{
    count::{count_segmented_records, count_single_records, Context, Counts},
    filter::Filter,
    specification::LibraryLayout,
};
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

    let min_mapping_quality = args.min_mapping_quality;
    let filter = Filter::new(min_mapping_quality);

    let mut reader = bam::io::reader::Builder.build_from_path(src)?;
    reader.read_header()?;

    info!("counting features");

    let ctx = match library_layout {
        LibraryLayout::Single => {
            count_single_records(&interval_trees, &filter, strand_specification, reader)?
        }
        LibraryLayout::Multiple => {
            count_segmented_records(&interval_trees, &filter, strand_specification, reader)?
        }
    };

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    let mut feature_names: Vec<_> = features.keys().collect();
    feature_names.sort();

    write_counts(&mut writer, &feature_names, &ctx.hits)?;
    write_metadata(&mut writer, &ctx)?;

    Ok(())
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

const DELIMITER: char = '\t';

fn write_counts<W>(writer: &mut W, feature_names: &[&String], counts: &Counts) -> io::Result<()>
where
    W: Write,
{
    const MISSING: u64 = 0;

    for name in feature_names {
        let count = counts.get(name.as_str()).copied().unwrap_or(MISSING);
        writeln!(writer, "{name}{DELIMITER}{count}")?;
    }

    Ok(())
}

fn write_metadata<W>(writer: &mut W, ctx: &Context) -> io::Result<()>
where
    W: Write,
{
    writeln!(writer, "__no_feature{DELIMITER}{}", ctx.miss)?;
    writeln!(writer, "__ambiguous{DELIMITER}{}", ctx.ambiguous)?;
    writeln!(writer, "__too_low_aQual{DELIMITER}{}", ctx.low_quality)?;
    writeln!(writer, "__not_aligned{DELIMITER}{}", ctx.unmapped)?;
    writeln!(writer, "__alignment_not_unique{DELIMITER}{}", ctx.nonunique)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_counts() -> io::Result<()> {
        let mut buf = Vec::new();

        let feature_names = [
            &String::from("f0"),
            &String::from("f1"),
            &String::from("f2"),
        ];

        let counts = [("f1", 8), ("f0", 13), ("f2", 5)].into_iter().collect();
        write_counts(&mut buf, &feature_names, &counts)?;

        assert_eq!(buf, b"f0\t13\nf1\t8\nf2\t5\n");

        Ok(())
    }

    #[test]
    fn test_write_metadata() -> io::Result<()> {
        let mut buf = Vec::new();

        let ctx = Context {
            hits: HashMap::new(),
            miss: 2,
            ambiguous: 3,
            low_quality: 5,
            unmapped: 8,
            nonunique: 13,
        };

        write_metadata(&mut buf, &ctx)?;

        let expected = b"\
__no_feature\t2
__ambiguous\t3
__too_low_aQual\t5
__not_aligned\t8
__alignment_not_unique\t13
";

        assert_eq!(buf, expected);

        Ok(())
    }
}
