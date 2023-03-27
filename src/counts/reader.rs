pub mod htseq_count;
pub mod star;

use std::collections::HashMap;

use tokio::io::{self, AsyncBufRead, AsyncBufReadExt};

use super::Format;
use crate::store::StrandSpecification;

pub async fn read_counts<R>(
    format: Format,
    feature_name: &str,
    strand_specification: StrandSpecification,
    reader: &mut R,
) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    match format {
        Format::HtseqCount => htseq_count::read_counts(reader).await,
        Format::Star => star::read_counts(feature_name, strand_specification, reader).await,
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
