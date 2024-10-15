use std::io::{self, BufRead};

use super::read_line;

const HTSEQ_COUNT_META_PREFIX: &str = "__";

pub(super) fn read<R>(reader: &mut R) -> io::Result<Vec<(String, u32)>>
where
    R: BufRead,
{
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

        let (name, count) = parse_line(&line)?;
        counts.push((name.into(), count));
    }

    Ok(counts)
}

#[allow(dead_code)]
pub(super) fn read_into<R>(
    reader: &mut R,
    names: &[String],
    counts: &mut Vec<u32>,
) -> io::Result<()>
where
    R: BufRead,
{
    let mut line = String::new();
    let mut expected_names = names.iter();

    while read_line(reader, &mut line)? != 0 {
        if line.starts_with(HTSEQ_COUNT_META_PREFIX) {
            break;
        }

        let (actual_name, count) = parse_line(&line)?;

        if let Some(expected_name) = expected_names.next() {
            if actual_name != expected_name {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid feature name: expected {expected_name}, got {actual_name}"),
                ));
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid feature name: expected None, got Some({actual_name})"),
            ));
        }

        counts.push(count);

        line.clear();
    }

    Ok(())
}

fn parse_line(s: &str) -> io::Result<(&str, u32)> {
    const DELIMITER: char = '\t';

    let (name, raw_count) = s
        .split_once(DELIMITER)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid input"))?;

    let count = raw_count
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok((name, count))
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
    fn test_read_into() -> io::Result<()> {
        let data = b"f0\t8\nf1\t13\n__no_feature\t0\nf2\t21\n";
        let mut reader = &data[..];

        let names = [String::from("f0"), String::from("f1")];
        let mut counts = Vec::new();
        read_into(&mut reader, &names, &mut counts)?;

        assert_eq!(counts, [8, 13]);

        Ok(())
    }

    #[test]
    fn test_parse_line() -> io::Result<()> {
        assert_eq!(parse_line("f0\t8")?, ("f0", 8));

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
