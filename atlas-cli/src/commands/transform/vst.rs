use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

use atlas_core::counts::transforms::vst;

use crate::cli;

pub fn run(args: cli::transform::vst::Args) -> anyhow::Result<()> {
    let (sample_names, feature_names, counts) = import(&args.src)?;

    let stabilized_counts = vst::transform(counts, feature_names.len(), sample_names.len());

    let mut writer = io::stdout().lock();

    for sample_name in &sample_names {
        write!(writer, "\t{sample_name}")?;
    }

    writeln!(writer)?;

    for (feature_name, row) in feature_names
        .iter()
        .zip(stabilized_counts.chunks_exact(sample_names.len()))
    {
        write!(writer, "{feature_name}")?;

        for n in row {
            write!(writer, "\t{n}")?;
        }

        writeln!(writer)?;
    }

    Ok(())
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
    const SEPARATOR: u8 = b'\t';

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(SEPARATOR)
        .from_reader(reader);

    let sample_names: Vec<_> = reader.headers()?.iter().skip(1).map(String::from).collect();
    let mut feature_names = Vec::new();
    let mut counts = Vec::new();

    for result in reader.records() {
        let row = result?;

        let mut fields = row.iter();

        let feature_name = fields
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing feature name"))?;

        feature_names.push(feature_name.into());

        for field in fields {
            let n = field.parse()?;
            counts.push(n);
        }
    }

    Ok((sample_names, feature_names, counts))
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
