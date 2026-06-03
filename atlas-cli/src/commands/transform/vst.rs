use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

use anyhow::bail;

use crate::cli;

pub fn run(args: cli::transform::vst::Args) -> anyhow::Result<()> {
    let (_sample_names, _feature_names, _counts) = import(&args.src)?;

    todo!()
}

fn import<P>(src: P) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<u32>)>
where
    P: AsRef<Path>,
{
    let mut reader = File::open(src).map(BufReader::new)?;
    import_inner(&mut reader)
}

fn import_inner<R>(reader: &mut R) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<u32>)>
where
    R: BufRead,
{
    const SEPARATOR: char = '\t';

    let mut line = String::new();
    let n = read_line(reader, &mut line)?;

    if n == 0 {
        bail!("input unexpectedly empty");
    }

    let sample_names: Vec<_> = line.split(SEPARATOR).skip(1).map(String::from).collect();
    let mut feature_names = Vec::new();
    let mut counts = Vec::new();

    loop {
        line.clear();
        let n = read_line(reader, &mut line)?;

        if n == 0 {
            break;
        }

        let mut row = line.split(SEPARATOR);

        let feature_name = row
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing feature name"))?;

        feature_names.push(feature_name.into());

        for field in row {
            let n = field.parse()?;
            counts.push(n);
        }
    }

    Ok((sample_names, feature_names, counts))
}

fn read_line<R>(reader: &mut R, dst: &mut String) -> io::Result<usize>
where
    R: BufRead,
{
    const LINE_FEED: char = '\n';
    const CARRIAGE_RETURN: char = '\r';

    match reader.read_line(dst)? {
        0 => Ok(0),
        n => {
            if dst.ends_with(LINE_FEED) {
                dst.pop();

                if dst.ends_with(CARRIAGE_RETURN) {
                    dst.pop();
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
    fn test_import() -> anyhow::Result<()> {
        let src = b"\ts0\ts1\nf0\t3\t5\nf1\t8\t13\n";

        let (sample_names, feature_names, counts) = import_inner(&mut &src[..])?;

        assert_eq!(sample_names, [String::from("s0"), String::from("s1")]);
        assert_eq!(feature_names, [String::from("f0"), String::from("f1")]);
        assert_eq!(counts, [3, 5, 8, 13]);

        Ok(())
    }
}
