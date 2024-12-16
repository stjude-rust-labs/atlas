mod feature;

use std::{
    collections::HashMap,
    io::{self, BufRead},
};

use indexmap::IndexSet;
use thiserror::Error;

pub use self::feature::Feature;

#[derive(Error, Debug)]
pub enum ReadFeaturesError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("missing attribute")]
    MissingAttribute,
    #[error("invalid attribute")]
    InvalidAttribute,
}

#[allow(clippy::type_complexity)]
pub fn read_features<R>(
    reader: &mut R,
    feature_type: &str,
    feature_id: &str,
) -> Result<(IndexSet<String>, HashMap<String, Vec<Feature>>), ReadFeaturesError>
where
    R: BufRead,
{
    use noodles::gff::{self, record::attributes::field::Value};

    let mut reference_sequence_names = IndexSet::new();
    let mut features: HashMap<String, Vec<Feature>> = HashMap::new();

    let mut reader = gff::io::Reader::new(reader);
    let mut line = gff::Line::default();

    while reader.read_line(&mut line)? != 0 {
        let Some(record) = line.as_record().transpose()? else {
            continue;
        };

        if record.ty() != feature_type {
            continue;
        }

        let reference_sequence_name = record.reference_sequence_name();

        let reference_sequence_id = match reference_sequence_names
            .get_index_of(reference_sequence_name)
        {
            Some(id) => id,
            None => {
                let (id, _) = reference_sequence_names.insert_full(reference_sequence_name.into());
                id
            }
        };

        let start = record.start()?;
        let end = record.end()?;
        let strand = record.strand()?;
        let feature = Feature::new(reference_sequence_id, start, end, strand);

        let attributes = record.attributes();
        let id = attributes
            .get(feature_id)
            .ok_or(ReadFeaturesError::MissingAttribute)?
            .map_err(|_| ReadFeaturesError::InvalidAttribute)
            .and_then(|value| match value {
                Value::String(s) => Ok(s),
                Value::Array(_) => Err(ReadFeaturesError::InvalidAttribute),
            })?;

        let segments = features.entry(id.into()).or_default();
        segments.push(feature);
    }

    Ok((reference_sequence_names, features))
}

pub fn merge_features(features: &[Feature]) -> Vec<Feature> {
    assert!(!features.is_empty());

    let mut features = features.to_vec();
    features.sort_unstable_by_key(|feature| feature.start);

    let mut merged_features = Vec::with_capacity(features.len());
    let mut current_feature = features[0].clone();

    for next_feature in features.iter().skip(1) {
        if next_feature.start > current_feature.end {
            merged_features.push(current_feature.clone());
            current_feature.start = next_feature.start;
            current_feature.end = next_feature.end;
        } else if current_feature.end < next_feature.end {
            current_feature.end = next_feature.end;
        }
    }

    merged_features.push(current_feature.clone());

    merged_features
}

pub fn calculate_feature_lengths(
    features: &HashMap<String, Vec<Feature>>,
    names: &[String],
) -> io::Result<Vec<usize>> {
    names
        .iter()
        .map(|name| {
            let segments = features
                .get(name)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing feature"))?;

            let merged_segments = merge_features(segments);

            let length = merged_segments
                .into_iter()
                .map(|feature| feature.length())
                .sum();

            Ok(length)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use noodles::{core::Position, gff::record_buf::Strand};

    use super::*;

    #[test]
    fn test_read_features() -> Result<(), Box<dyn std::error::Error>> {
        const DATA: &[u8] = b"\
##gff-version 3
sq0	.	exon	1	5	.	+	.	ID=1.0;gene_name=r1
sq0	.	exon	3	8	.	+	.	ID=1.1;gene_name=r1
sq0	.	gene	1	8	.	.	.	ID=2.0;gene_name=r1
sq0	.	exon	13	21	.	-	.	ID=3.0;gene_name=r2
";

        let mut reader = DATA;
        let (_, actual) = read_features(&mut reader, "exon", "gene_name")?;

        let expected = [
            (
                String::from("r1"),
                vec![
                    Feature::new(
                        0,
                        Position::try_from(1)?,
                        Position::try_from(5)?,
                        Strand::Forward,
                    ),
                    Feature::new(
                        0,
                        Position::try_from(3)?,
                        Position::try_from(8)?,
                        Strand::Forward,
                    ),
                ],
            ),
            (
                String::from("r2"),
                vec![Feature::new(
                    0,
                    Position::try_from(13)?,
                    Position::try_from(21)?,
                    Strand::Reverse,
                )],
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_merge_features() -> Result<(), noodles::core::position::TryFromIntError> {
        const STRAND: Strand = Strand::None;

        let features = [
            Feature::new(0, Position::try_from(2)?, Position::try_from(5)?, STRAND),
            Feature::new(0, Position::try_from(3)?, Position::try_from(4)?, STRAND),
            Feature::new(0, Position::try_from(5)?, Position::try_from(7)?, STRAND),
            Feature::new(0, Position::try_from(9)?, Position::try_from(12)?, STRAND),
            Feature::new(0, Position::try_from(10)?, Position::try_from(15)?, STRAND),
            Feature::new(0, Position::try_from(16)?, Position::try_from(21)?, STRAND),
        ];

        let actual = merge_features(&features);

        let expected = [
            Feature::new(0, Position::try_from(2)?, Position::try_from(7)?, STRAND),
            Feature::new(0, Position::try_from(9)?, Position::try_from(15)?, STRAND),
            Feature::new(0, Position::try_from(16)?, Position::try_from(21)?, STRAND),
        ];

        assert_eq!(actual, expected);

        Ok(())
    }
}
