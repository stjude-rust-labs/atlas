mod htseq_count;

use std::io::{self, BufRead};

pub fn read<R>(reader: &mut R) -> io::Result<Vec<(String, u64)>>
where
    R: BufRead,
{
    htseq_count::read(reader)
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
