use std::io::{self, BufRead};

use super::read_line;
use crate::StrandSpecification;

pub(super) fn read<R>(
    reader: &mut R,
    feature_name: &str,
    strand_specification: StrandSpecification,
) -> io::Result<Vec<(String, u32)>>
where
    R: BufRead,
{
    let name_index = match feature_name {
        "gene_id" => 0,
        "gene_name" => 1,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid feature name",
            ))
        }
    };

    let count_index = match strand_specification {
        StrandSpecification::None => 3,
        StrandSpecification::Forward => 4,
        StrandSpecification::Reverse => 5,
    };

    let mut line = String::new();
    let mut counts = Vec::new();

    consume_meta(reader, &mut line)?;

    loop {
        line.clear();

        if read_line(reader, &mut line)? == 0 {
            break;
        }

        let (name, count) = parse_line(&line, name_index, count_index)?;
        counts.push((name.into(), count));
    }

    Ok(counts)
}

fn consume_meta<R>(reader: &mut R, buf: &mut String) -> io::Result<()>
where
    R: BufRead,
{
    const META_LINE_COUNT: usize = 6;

    for _ in 0..META_LINE_COUNT {
        buf.clear();
        read_line(reader, buf)?;
    }

    Ok(())
}

fn parse_line(s: &str, name_index: usize, count_index: usize) -> io::Result<(&str, u32)> {
    const COLUMN_COUNT: usize = 9;
    const DELIMITER: char = '\t';

    assert!(count_index >= 3);

    // SAFETY: `count_index` is at minimum 3.
    let count_offset = count_index - name_index - 1;

    let mut fields = s.splitn(COLUMN_COUNT, DELIMITER);

    let name = fields
        .nth(name_index)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing name column"))?;

    let raw_count = fields
        .nth(count_offset)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing count column"))?;

    let count = raw_count
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok((name, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read() -> io::Result<()> {
        const DATA: &[u8] = b"\
# gene-model: GENCODE v46
gene_id\tgene_name\tgene_type\tunstranded\tstranded_first\tstranded_second\ttpm_unstranded\tfpkm_unstranded\tfpkm_uq_unstranded
N_unmapped\t\t\t0\t0\t0\t\t\t
N_multimapping\t\t\t0\t0\t0\t\t\t
N_noFeature\t\t\t0\t0\t0\t\t\t
N_ambiguous\t\t\t0\t0\t0\t\t\t
A0.1\tf0\tprotein_coding\t21\t13\t8\t0.0\t0.0\t0.0
A1.1\tf1\tprotein_coding\t89\t55\t34\t0.0\t0.0\t0.0
";

        let mut reader = DATA;
        let actual = read(&mut reader, "gene_name", StrandSpecification::None)?;
        let expected = [(String::from("f0"), 21), (String::from("f1"), 89)];
        assert_eq!(actual, expected);

        let mut reader = DATA;
        let actual = read(&mut reader, "gene_name", StrandSpecification::Forward)?;
        let expected = [(String::from("f0"), 13), (String::from("f1"), 55)];
        assert_eq!(actual, expected);

        let mut reader = DATA;
        let actual = read(&mut reader, "gene_id", StrandSpecification::Reverse)?;
        let expected = [(String::from("A0.1"), 8), (String::from("A1.1"), 34)];
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_parse_line() -> io::Result<()> {
        let s = "A0.1\tf0\tprotein_coding\t21\t13\t8\t0.0\t0.0\t0.0";
        assert_eq!(parse_line(s, 0, 3)?, ("A0.1", 21));
        assert_eq!(parse_line(s, 0, 4)?, ("A0.1", 13));
        assert_eq!(parse_line(s, 1, 5)?, ("f0", 8));

        // missing name
        assert!(matches!(
            parse_line("A0.1", 1, 3),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        // missing count
        assert!(matches!(
            parse_line("A0.1\tf0\tprotein_coding", 1, 3),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        // invalid count
        assert!(matches!(
            parse_line("A0.1\tf0\tprotein_coding\tatlas", 1, 3),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        Ok(())
    }
}
