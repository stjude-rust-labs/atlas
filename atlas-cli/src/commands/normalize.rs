use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    num,
    path::Path,
};

use atlas_core::{
    counts::normalization::{fpkm, median_of_ratios, tpm},
    features::{calculate_feature_lengths, Feature, ReadFeaturesError},
    StrandSpecification,
};
use thiserror::Error;

use crate::cli::normalize::{self, Method};

pub fn normalize(args: normalize::Args) -> Result<(), NormalizeError> {
    let features = read_features(&args.annotations, &args.feature_type, &args.feature_id)?;
    let feature_counts = read_counts(&args.src, &args.feature_id, StrandSpecification::None)?; // FIXME: strand specification

    let counts: Vec<_> = feature_counts.iter().map(|(_, n)| *n).collect();
    let names: Vec<_> = feature_counts.into_iter().map(|(name, _)| name).collect();

    let normalized_counts = match args.method {
        Method::Fpkm => {
            let feature_lengths: Vec<_> = calculate_feature_lengths(&features, &names)?
                .into_iter()
                .map(|length| length as i32)
                .collect();

            let counts: Vec<_> = counts.into_iter().map(|value| value as i32).collect();

            vec![fpkm::normalize(&feature_lengths, &counts)]
        }
        Method::MedianOfRatios => {
            let data = counts
                .into_iter()
                .map(u32::try_from)
                .collect::<Result<_, num::TryFromIntError>>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            median_of_ratios::normalize_vec(1, names.len(), data)?
        }
        Method::Tpm => {
            let feature_lengths: Vec<_> = calculate_feature_lengths(&features, &names)?
                .into_iter()
                .map(|length| length as i32)
                .collect();

            let counts: Vec<_> = counts.into_iter().map(|value| value as i32).collect();

            vec![tpm::normalize(&feature_lengths, &counts)]
        }
    };

    write_normalized_counts(&names, &normalized_counts[0])?;

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

fn write_normalized_counts(feature_names: &[String], normalized_counts: &[f64]) -> io::Result<()> {
    const SEPARATOR: char = '\t';

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    for (name, value) in feature_names.iter().zip(normalized_counts) {
        writeln!(writer, "{name}{SEPARATOR}{value}")?;
    }

    Ok(())
}
