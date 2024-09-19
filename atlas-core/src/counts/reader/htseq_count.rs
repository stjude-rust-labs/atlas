use std::io::{self, BufRead};

use super::read_line;

pub(super) fn read<R>(reader: &mut R) -> io::Result<Vec<(String, u64)>>
where
    R: BufRead,
{
    const HTSEQ_COUNT_META_PREFIX: &str = "__";

    let mut line = String::new();
    let mut counts = Vec::new();

    loop {
        line.clear();

        if read_line(reader, &mut line)? == 0 {
            break;
        }

        if line.starts_with(HTSEQ_COUNT_META_PREFIX) {
            break;
        }

        let entry = parse_line(&line)?;
        counts.push(entry);
    }

    Ok(counts)
}

fn parse_line(s: &str) -> io::Result<(String, u64)> {
    const DELIMITER: char = '\t';

    let (raw_name, raw_count) = s
        .split_once(DELIMITER)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid input"))?;

    let count = raw_count
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok((raw_name.into(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_counts() -> io::Result<()> {
        let data = b"f0\t8\nf1\t13\n__no_feature\t0\nf2\t21\n";
        let mut reader = &data[..];
        let actual = read(&mut reader)?;
        let expected = [(String::from("f0"), 8), (String::from("f1"), 13)];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_parse_line() -> io::Result<()> {
        assert_eq!(parse_line("f0\t8")?, (String::from("f0"), 8));

        assert!(matches!(
            parse_line("f0 13"),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        assert!(matches!(
            parse_line("f0 13"),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        Ok(())
    }
}
