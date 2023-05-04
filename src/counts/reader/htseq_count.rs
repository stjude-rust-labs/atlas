use std::{collections::HashMap, num};

use thiserror::Error;
use tokio::io::AsyncBufRead;

use super::read_line;

pub async fn read_counts<R>(reader: &mut R) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    const HTSEQ_COUNT_META_PREFIX: &str = "__";

    let mut line = String::new();
    let mut counts = HashMap::new();

    loop {
        line.clear();

        if read_line(reader, &mut line).await? == 0 {
            break;
        }

        let (name, count) = parse_line(&line)?;

        if name.starts_with(HTSEQ_COUNT_META_PREFIX) {
            break;
        }

        counts.insert(name, count);
    }

    Ok(counts)
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParseError {
    #[error("invalid input")]
    Invalid,
    #[error("invalid count")]
    InvalidCount(num::ParseIntError),
}

fn parse_line(s: &str) -> Result<(String, u64), ParseError> {
    const DELIMITER: char = '\t';
    let (raw_name, raw_count) = s.split_once(DELIMITER).ok_or(ParseError::Invalid)?;
    let count = raw_count.parse().map_err(ParseError::InvalidCount)?;
    Ok((raw_name.into(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_counts() -> anyhow::Result<()> {
        let data = b"feature_1\t8\nfeature_2\t13\n__no_feature\t0";

        let mut reader = &data[..];
        let counts = read_counts(&mut reader).await?;

        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 8);
        assert_eq!(counts["feature_2"], 13);

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_line() {
        assert_eq!(
            parse_line("feature_1\t8"),
            Ok((String::from("feature_1"), 8))
        );

        assert_eq!(parse_line("feature_2 13"), Err(ParseError::Invalid));

        assert!(matches!(
            parse_line("feature_3\tone"),
            Err(ParseError::InvalidCount(_))
        ));
    }
}
