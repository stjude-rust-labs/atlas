pub mod htseq_count;
pub mod star;

use std::collections::HashMap;

use tokio::io::{self, AsyncBufRead, AsyncBufReadExt};
use tracing::warn;

use super::Format;
use crate::store::StrandSpecification;

pub async fn read_counts<R>(
    reader: &mut R,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    let detected_format = detect_format(reader).await?;

    if let Some(format) = format {
        if format != detected_format {
            warn!(
                expected = ?detected_format,
                actual = ?format,
                "format mismatch"
            );
        }
    }

    let format = format.unwrap_or(detected_format);

    match format {
        Format::HtseqCount => htseq_count::read_counts(reader).await,
        Format::Star => star::read_counts(reader, feature_name, strand_specification).await,
    }
}

async fn detect_format<R>(reader: &mut R) -> io::Result<Format>
where
    R: AsyncBufRead + Unpin,
{
    const STAR_FORMAT_PREFIX: &[u8] = b"# gene-model:";

    let src = reader.fill_buf().await?;

    if src.starts_with(STAR_FORMAT_PREFIX) {
        Ok(Format::Star)
    } else {
        Ok(Format::HtseqCount)
    }
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
    async fn test_detect_format() -> io::Result<()> {
        let src = b"# gene-model: GENCODE v43\n";
        let mut reader = &src[..];
        assert_eq!(detect_format(&mut reader).await?, Format::Star);

        let src = b"feature_1\t8\n";
        let mut reader = &src[..];
        assert_eq!(detect_format(&mut reader).await?, Format::HtseqCount);

        let src = b"atlas\n";
        let mut reader = &src[..];
        assert_eq!(detect_format(&mut reader).await?, Format::HtseqCount);

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
