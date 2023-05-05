use std::{collections::HashMap, num};

use thiserror::Error;
use tokio::io::{self, AsyncBufRead};

use super::read_line;
use crate::store::StrandSpecification;

pub async fn read_counts<R>(
    feature_name: &str,
    strand_specification: StrandSpecification,
    reader: &mut R,
) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    let name_index = match feature_name {
        "gene_id" => 0,
        "gene_name" => 1,
        _ => anyhow::bail!("invalid feature name: {}", feature_name),
    };

    let count_index = match strand_specification {
        StrandSpecification::None => 3,
        StrandSpecification::Forward => 4,
        StrandSpecification::Reverse => 5,
    };

    let mut line = String::new();
    let mut counts = HashMap::new();

    consume_meta(reader, &mut line).await?;

    loop {
        line.clear();

        if read_line(reader, &mut line).await? == 0 {
            break;
        }

        let (name, count) = parse_line(&line, name_index, count_index)?;
        counts.insert(name, count);
    }

    Ok(counts)
}

async fn consume_meta<R>(reader: &mut R, buf: &mut String) -> io::Result<()>
where
    R: AsyncBufRead + Unpin,
{
    const META_LINE_COUNT: usize = 6;

    for _ in 0..META_LINE_COUNT {
        buf.clear();
        read_line(reader, buf).await?;
    }

    Ok(())
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParseError {
    #[error("missing name in column {column_index}")]
    MissingName { column_index: usize },
    #[error("missing count in column {column_index}")]
    MissingCount { column_index: usize },
    #[error("invalid count")]
    InvalidCount(num::ParseIntError),
}

fn parse_line(s: &str, name_index: usize, count_index: usize) -> Result<(String, u64), ParseError> {
    const COLUMN_COUNT: usize = 9;
    const DELIMITER: char = '\t';

    assert!(count_index >= 3);

    // SAFETY: `count_index` is at minimum 3.
    let count_offset = count_index - name_index - 1;

    let mut fields = s.splitn(COLUMN_COUNT, DELIMITER);

    let raw_name = fields.nth(name_index).ok_or(ParseError::MissingName {
        column_index: name_index,
    })?;

    let raw_count = fields.nth(count_offset).ok_or(ParseError::MissingCount {
        column_index: count_index,
    })?;

    let count = raw_count.parse().map_err(ParseError::InvalidCount)?;

    Ok((raw_name.into(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_counts() -> anyhow::Result<()> {
        let data = b"\
# gene-model: GENCODE v43
gene_id\tgene_name\tgene_type\tunstranded\tstranded_first\tstranded_second\ttpm_unstranded\tfpkm_unstranded\tfpkm_uq_unstranded
N_unmapped\t\t\t0\t0\t0\t\t\t
N_multimapping\t\t\t0\t0\t0\t\t\t
N_noFeature\t\t\t0\t0\t0\t\t\t
N_ambiguous\t\t\t0\t0\t0\t\t\t
ATLAS1.1\tfeature_1\tprotein_coding\t21\t13\t8\t0.0\t0.0\t0.0
ATLAS2.1\tfeature_2\tprotein_coding\t89\t55\t34\t0.0\t0.0\t0.0
";

        let mut reader = &data[..];
        let counts = read_counts("gene_name", StrandSpecification::None, &mut reader).await?;
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 21);
        assert_eq!(counts["feature_2"], 89);

        let mut reader = &data[..];
        let counts = read_counts("gene_name", StrandSpecification::Forward, &mut reader).await?;
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 13);
        assert_eq!(counts["feature_2"], 55);

        let mut reader = &data[..];
        let counts = read_counts("gene_id", StrandSpecification::Reverse, &mut reader).await?;
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["ATLAS1.1"], 8);
        assert_eq!(counts["ATLAS2.1"], 34);

        Ok(())
    }

    #[test]
    fn test_parse_line() {
        let s = "ATLAS1.1\tfeature_1\tprotein_coding\t21\t13\t8\t0.0\t0.0\t0.0";
        assert_eq!(parse_line(s, 0, 3), Ok((String::from("ATLAS1.1"), 21)));
        assert_eq!(parse_line(s, 0, 4), Ok((String::from("ATLAS1.1"), 13)));
        assert_eq!(parse_line(s, 1, 5), Ok((String::from("feature_1"), 8)));

        let s = "ATLAS1.1";
        assert_eq!(
            parse_line(s, 1, 3),
            Err(ParseError::MissingName { column_index: 1 })
        );

        let s = "ATLAS1.1\tfeature_1\tprotein_coding";
        assert_eq!(
            parse_line(s, 0, 3),
            Err(ParseError::MissingCount { column_index: 3 })
        );

        let s = "ATLAS1.1\tfeature_1\tprotein_coding\tatlas\t13\t8\t0.0\t0.0\t0.0";
        assert!(matches!(
            parse_line(s, 0, 3),
            Err(ParseError::InvalidCount(_))
        ));
    }
}
