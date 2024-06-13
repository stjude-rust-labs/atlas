#![allow(dead_code)]

use std::{
    collections::HashMap,
    io::{self, BufRead},
};

use noodles::core::Position;
use thiserror::Error;

pub type Feature = (Position, Position);

#[derive(Error, Debug)]
pub enum ReadFeaturesError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("missing attribute")]
    MissingAttribute,
    #[error("invalid attribute")]
    InvalidAttribute,
}

pub fn read_features<R>(
    reader: &mut R,
    feature_type: &str,
    feature_id: &str,
) -> Result<HashMap<String, Vec<Feature>>, ReadFeaturesError>
where
    R: BufRead,
{
    use noodles::gff::{
        self,
        lazy::{record::attributes::field::Value, Line},
    };

    let mut features: HashMap<String, Vec<Feature>> = HashMap::new();

    let mut reader = gff::io::Reader::new(reader);
    let mut line = Line::default();

    while reader.read_lazy_line(&mut line)? != 0 {
        let Line::Record(ref record) = line else {
            continue;
        };

        if record.ty() != feature_type {
            continue;
        }

        let start = record.start()?;
        let end = record.end()?;
        let feature = (start, end);

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

    Ok(features)
}

pub fn merge_features(features: &[Feature]) -> Vec<Feature> {
    assert!(!features.is_empty());

    let mut features = features.to_vec();
    features.sort_unstable();

    let mut merged_features = Vec::with_capacity(features.len());
    let (mut current_start, mut current_end) = features[0];

    for (next_start, next_end) in features.iter().copied().skip(1) {
        if next_start > current_end {
            merged_features.push((current_start, current_end));
            current_start = next_start;
            current_end = next_end;
        } else if current_end < next_end {
            current_end = next_end;
        }
    }

    merged_features.push((current_start, current_end));

    merged_features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_features() -> Result<(), Box<dyn std::error::Error>> {
        const DATA: &[u8] = b"\
##gff-version 3
sq0	.	exon	1	5	.	.	.	ID=1.0;gene_name=r1
sq0	.	exon	3	8	.	.	.	ID=1.1;gene_name=r1
sq0	.	gene	1	8	.	.	.	ID=2.0;gene_name=r1
sq0	.	exon	13	21	.	.	.	ID=3.0;gene_name=r2
";

        let mut reader = DATA;
        let actual = read_features(&mut reader, "exon", "gene_name")?;

        let expected = [
            (
                String::from("r1"),
                vec![
                    (Position::try_from(1)?, Position::try_from(5)?),
                    (Position::try_from(3)?, Position::try_from(8)?),
                ],
            ),
            (
                String::from("r2"),
                vec![(Position::try_from(13)?, Position::try_from(21)?)],
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_merge_features() -> Result<(), noodles::core::position::TryFromIntError> {
        let features = [
            (Position::try_from(2)?, Position::try_from(5)?),
            (Position::try_from(3)?, Position::try_from(4)?),
            (Position::try_from(5)?, Position::try_from(7)?),
            (Position::try_from(9)?, Position::try_from(12)?),
            (Position::try_from(10)?, Position::try_from(15)?),
            (Position::try_from(16)?, Position::try_from(21)?),
        ];

        let actual = merge_features(&features);

        let expected = [
            (Position::try_from(2)?, Position::try_from(7)?),
            (Position::try_from(9)?, Position::try_from(15)?),
            (Position::try_from(16)?, Position::try_from(21)?),
        ];

        assert_eq!(actual, expected);

        Ok(())
    }
}
