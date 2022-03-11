use std::collections::HashMap;

use tokio::io::{AsyncBufRead, AsyncBufReadExt};

#[allow(dead_code)]
async fn read_feature_counts<R>(reader: &mut R) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    const TAB: char = '\t';

    let mut lines = reader.lines();
    let mut counts = HashMap::new();

    while let Some(line) = lines.next_line().await? {
        if let Some((raw_name, raw_count)) = line.split_once(TAB) {
            let count = raw_count.parse()?;
            counts.insert(raw_name.into(), count);
        }
    }

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_feature_counts() -> anyhow::Result<()> {
        let data = b"feature_1\t8\nfeature_2\t13\n";

        let mut reader = &data[..];
        let counts = read_feature_counts(&mut reader).await?;

        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 8);
        assert_eq!(counts["feature_2"], 13);

        Ok(())
    }
}
