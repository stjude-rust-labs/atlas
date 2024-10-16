use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use atlas_core::{
    counts::normalization::{fpkm, median_of_ratios, tpm},
    features::{self, Feature, ReadFeaturesError},
    StrandSpecification,
};
use thiserror::Error;
use tracing::info;

use crate::cli::normalize::{self, Method};

const SEPARATOR: char = '\t';

pub fn normalize(args: normalize::Args) -> Result<(), NormalizeError> {
    let annotations_src = &args.annotations;
    let feature_type = &args.feature_type;
    let feature_id = &args.feature_id;

    info!(
        src = ?annotations_src,
        feature_type, feature_id, "reading features"
    );

    let features = read_features(annotations_src, feature_type, feature_id)?;

    info!(feature_count = features.len(), "read features");

    let format = args.format.map(|format| format.into());
    let strand_specification = StrandSpecification::from(args.strand_specification);

    let sample_count = args.srcs.len();
    let mut srcs = args.srcs.iter();

    info!(sample_count, "reading samples");

    // SAFETY: `srcs` is nonempty.
    let src = srcs.next().unwrap();
    let counts = read_counts(src, format, feature_id, strand_specification)?;

    let names: Vec<_> = counts.iter().map(|(name, _)| name.clone()).collect();
    let mut counts: Vec<_> = counts.into_iter().map(|(_, value)| value).collect();

    for src in srcs {
        read_counts_into(
            src,
            format,
            feature_id,
            strand_specification,
            &names,
            &mut counts,
        )?;
    }

    let normalization_method = args.method;

    info!(?normalization_method, "normalizing counts");

    let normalized_counts: Vec<Vec<f64>> = match normalization_method {
        Method::Fpkm => {
            let lengths = calculate_feature_lengths(&features, &names)?;

            counts
                .chunks_exact(names.len())
                .map(|sample| fpkm::normalize(&lengths, sample))
                .collect()
        }
        Method::MedianOfRatios => {
            median_of_ratios::normalize_vec(sample_count, names.len(), counts)?
        }
        Method::Tpm => {
            let lengths = calculate_feature_lengths(&features, &names)?;

            counts
                .chunks_exact(names.len())
                .map(|sample| tpm::normalize(&lengths, sample))
                .collect()
        }
    };

    assert!(!normalized_counts.is_empty());

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    if normalized_counts.len() > 1 {
        write_multi_sample_normalized_counts(&mut writer, &args.srcs, &names, &normalized_counts)?;
    } else {
        write_single_sample_normalized_counts(&mut writer, &names, &normalized_counts[0])?;
    }

    info!("done");

    Ok(())
}

#[derive(Debug, Error)]
pub enum NormalizeError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("invalid features")]
    InvalidFeatures(#[from] ReadFeaturesError),
}

fn read_features<P>(
    src: P,
    feature_type: &str,
    feature_id: &str,
) -> Result<HashMap<String, Vec<Feature>>, NormalizeError>
where
    P: AsRef<Path>,
{
    use atlas_core::features::read_features;

    let mut reader = File::open(src).map(BufReader::new)?;
    let features = read_features(&mut reader, feature_type, feature_id)?;
    Ok(features)
}

fn read_counts<P>(
    src: P,
    format: Option<atlas_core::counts::reader::Format>,
    feature_id: &str,
    strand_specification: StrandSpecification,
) -> io::Result<Vec<(String, u32)>>
where
    P: AsRef<Path>,
{
    use atlas_core::counts::reader;

    let mut reader = File::open(src).map(BufReader::new)?;
    reader::read(&mut reader, format, feature_id, strand_specification)
}

fn read_counts_into<P>(
    src: P,
    format: Option<atlas_core::counts::reader::Format>,
    feature_id: &str,
    strand_specification: StrandSpecification,
    feature_names: &[String],
    dst: &mut Vec<u32>,
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    use atlas_core::counts::reader;

    let mut reader = File::open(src).map(BufReader::new)?;

    reader::read_into(
        &mut reader,
        format,
        feature_names,
        feature_id,
        strand_specification,
        dst,
    )
}

fn calculate_feature_lengths(
    features: &HashMap<String, Vec<Feature>>,
    names: &[String],
) -> io::Result<Vec<u32>> {
    features::calculate_feature_lengths(features, names)?
        .into_iter()
        .map(|length| {
            u32::try_from(length).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
        .collect()
}

fn write_multi_sample_normalized_counts<W>(
    writer: &mut W,
    srcs: &[PathBuf],
    feature_names: &[String],
    normalized_counts: &[Vec<f64>],
) -> io::Result<()>
where
    W: Write,
{
    for src in srcs {
        let basename = src
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid sample name"))?;

        write!(writer, "{SEPARATOR}{basename}")?;
    }

    writeln!(writer)?;

    for (i, name) in feature_names.iter().enumerate() {
        write!(writer, "{name}")?;

        for counts in normalized_counts {
            let value = counts[i];
            write!(writer, "{SEPARATOR}{value}")?;
        }

        writeln!(writer)?;
    }

    Ok(())
}

fn write_single_sample_normalized_counts<W>(
    writer: &mut W,
    feature_names: &[String],
    normalized_counts: &[f64],
) -> io::Result<()>
where
    W: Write,
{
    for (name, value) in feature_names.iter().zip(normalized_counts) {
        writeln!(writer, "{name}{SEPARATOR}{value}")?;
    }

    Ok(())
}
