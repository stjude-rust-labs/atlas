use std::collections::HashMap;

use tokio::io::AsyncBufRead;

use super::read_line;

pub async fn read_counts<R>(reader: &mut R) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    const DELIMITER: char = '\t';
    const HTSEQ_COUNT_META_PREFIX: &str = "__";

    let mut line = String::new();
    let mut counts = HashMap::new();

    loop {
        line.clear();

        if read_line(reader, &mut line).await? == 0 {
            break;
        }

        if let Some((raw_name, raw_count)) = line.split_once(DELIMITER) {
            if raw_name.starts_with(HTSEQ_COUNT_META_PREFIX) {
                break;
            }

            let count = raw_count.parse()?;

            counts.insert(raw_name.into(), count);
        } else {
            anyhow::bail!("invalid feature count line");
        }
    }

    Ok(counts)
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
    async fn test_read_counts_with_invalid_line() -> anyhow::Result<()> {
        let data = b"feature_1\t8\nfeature_2  13\n";
        let mut reader = &data[..];
        assert!(read_counts(&mut reader).await.is_err());
        Ok(())
    }
}
