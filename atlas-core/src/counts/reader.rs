mod format;
mod htseq_count;
mod star;

use std::io::{self, BufRead};

use tracing::warn;

pub use self::format::Format;
use crate::StrandSpecification;

pub fn read<R>(
    reader: &mut R,
    format: Option<Format>,
    feature_name: &str,
    strand_specification: StrandSpecification,
) -> io::Result<Vec<(String, u32)>>
where
    R: BufRead,
{
    let detected_format = detect_format(reader)?;

    if let Some(expected_format) = format {
        if detected_format != expected_format {
            warn!(
                expected = ?expected_format,
                actual = ?detected_format,
                "format mismatch"
            );
        }
    }

    let format = format.unwrap_or(detected_format);

    match format {
        Format::HtseqCount => htseq_count::read(reader),
        Format::Star => star::read(reader, feature_name, strand_specification),
    }
}

fn detect_format<R>(reader: &mut R) -> io::Result<Format>
where
    R: BufRead,
{
    const STAR_FORMAT_PREFIX: &[u8] = b"# gene-model:";

    let src = reader.fill_buf()?;

    if src.starts_with(STAR_FORMAT_PREFIX) {
        Ok(Format::Star)
    } else {
        Ok(Format::HtseqCount)
    }
}

fn read_line<R>(reader: &mut R, buf: &mut String) -> io::Result<usize>
where
    R: BufRead,
{
    const LINE_FEED: char = '\n';
    const CARRIAGE_RETURN: char = '\r';

    match reader.read_line(buf)? {
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

    #[test]
    fn test_detect_format() -> io::Result<()> {
        fn t(mut src: &[u8], expected: Format) -> io::Result<()> {
            let actual = detect_format(&mut src)?;
            assert_eq!(actual, expected);
            Ok(())
        }

        t(b"# gene-model: GENCODE v46\n", Format::Star)?;
        t(b"f0\t8\n", Format::HtseqCount)?;
        t(b"atlas\n", Format::HtseqCount)?;

        Ok(())
    }

    #[test]
    fn test_read_line() -> io::Result<()> {
        fn t(buf: &mut String, mut data: &[u8], expected: &str) -> io::Result<()> {
            buf.clear();
            read_line(&mut data, buf)?;
            assert_eq!(buf, expected);
            Ok(())
        }

        let mut buf = String::new();

        t(&mut buf, b"atlas\n", "atlas")?;
        t(&mut buf, b"atlas\r\n", "atlas")?;
        t(&mut buf, b"atlas", "atlas")?;

        Ok(())
    }
}
