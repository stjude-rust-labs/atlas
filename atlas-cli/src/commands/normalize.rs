use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use atlas_core::{
    counts::normalization::{fpkm, median_of_ratios, tpm},
    features::{calculate_feature_lengths, Feature, ReadFeaturesError},
    StrandSpecification,
};
use thiserror::Error;

use crate::cli::normalize::{self, Method};

const SEPARATOR: char = '\t';

pub fn normalize(args: normalize::Args) -> Result<(), NormalizeError> {
    let features = read_features(&args.annotations, &args.feature_type, &args.feature_id)?;

    let samples: Vec<_> = args
        .srcs
        .iter()
        .map(|src| {
            read_counts(src, &args.feature_id, StrandSpecification::None) // FIXME: strand specification
        })
        .collect::<io::Result<_>>()?;

    assert!(!samples.is_empty());

    let names: Vec<_> = samples[0].iter().map(|(name, _)| name.clone()).collect();

    let normalized_counts: Vec<Vec<f64>> = match args.method {
        Method::Fpkm => {
            let feature_lengths: Vec<_> = calculate_feature_lengths(&features, &names)?
                .into_iter()
                .map(|length| length as i32)
                .collect();

            samples
                .iter()
                .map(|sample| {
                    let counts: Vec<_> = sample.iter().map(|(_, value)| *value as i32).collect();
                    fpkm::normalize(&feature_lengths, &counts)
                })
                .collect()
        }
        Method::MedianOfRatios => {
            let counts = samples
                .iter()
                .flat_map(|sample| sample.iter().map(|(_, n)| *n as u32))
                .collect();

            median_of_ratios::normalize_vec(samples.len(), names.len(), counts)?
        }
        Method::Tpm => {
            let feature_lengths: Vec<_> = calculate_feature_lengths(&features, &names)?
                .into_iter()
                .map(|length| length as i32)
                .collect();

            samples
                .iter()
                .map(|sample| {
                    let counts: Vec<_> = sample.iter().map(|(_, value)| *value as i32).collect();
                    tpm::normalize(&feature_lengths, &counts)
                })
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
    feature_id: &str,
    strand_specification: StrandSpecification,
) -> io::Result<Vec<(String, u64)>>
where
    P: AsRef<Path>,
{
    use atlas_core::counts::reader;

    let mut reader = File::open(src).map(BufReader::new)?;
    reader::read(&mut reader, None, feature_id, strand_specification)
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
