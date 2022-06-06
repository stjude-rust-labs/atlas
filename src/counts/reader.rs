use std::collections::HashMap;

use tokio::io::{self, AsyncBufRead, AsyncBufReadExt};

pub async fn read_feature_counts<R>(reader: &mut R) -> anyhow::Result<HashMap<String, u64>>
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

async fn read_line<R>(reader: &mut R, buf: &mut String) -> io::Result<usize>
where
    R: AsyncBufRead + Unpin,
{
    const LINE_FEED: char = '\n';
    const CARRIAGE_RETURN: char = '\r';

    match reader.read_line(buf).await? {
        0 => Ok(0),
        n => {
            if buf.ends_with(LINE_FEED) {
                buf.pop();

                if buf.ends_with(CARRIAGE_RETURN) {
                    buf.pop();
                }
            }

            Ok(n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_feature_counts() -> anyhow::Result<()> {
        let data = b"feature_1\t8\nfeature_2\t13\n__no_feature\t0";

        let mut reader = &data[..];
        let counts = read_feature_counts(&mut reader).await?;

        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 8);
        assert_eq!(counts["feature_2"], 13);

        Ok(())
    }

    #[tokio::test]
    async fn test_read_feature_counts_with_invalid_line() -> anyhow::Result<()> {
        let data = b"feature_1\t8\nfeature_2  13\n";
        let mut reader = &data[..];
        assert!(read_feature_counts(&mut reader).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_read_line() -> io::Result<()> {
        async fn t(buf: &mut String, mut data: &[u8], expected: &str) -> io::Result<()> {
            buf.clear();
            read_line(&mut data, buf).await?;
            assert_eq!(buf, expected);
            Ok(())
        }

        let mut buf = String::new();

        t(&mut buf, b"atlas\n", "atlas").await?;
        t(&mut buf, b"atlas\r\n", "atlas").await?;
        t(&mut buf, b"atlas", "atlas").await?;

        Ok(())
    }
}
