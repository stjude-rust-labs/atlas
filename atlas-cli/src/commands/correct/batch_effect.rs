use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

use atlas_core::counts::correction::batch_effect;

use crate::cli;

const SEPARATOR: u8 = b'\t';

pub fn run(args: cli::correct::batch_effect::Args) -> anyhow::Result<()> {
    let (sample_names, feature_names, counts) = import(&args.src)?;
    let metadata = read_metadata(&args.metadata)?;

    let sample_count = sample_names.len();
    let feature_count = feature_names.len();
    let raw_corrected_counts =
        batch_effect::correct_raw(&counts, feature_count, sample_count, &metadata);

    let mut writer = io::stdout().lock();

    export(
        &mut writer,
        &sample_names,
        &feature_names,
        &raw_corrected_counts,
    )?;

    Ok(())
}

fn import<P>(src: P) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<f64>)>
where
    P: AsRef<Path>,
{
    let mut reader = File::open(src).map(BufReader::new)?;
    import_inner(&mut reader)
}

fn import_inner<R>(reader: &mut R) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<f64>)>
where
    R: BufRead,
{
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

fn read_metadata<P>(src: P) -> anyhow::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(SEPARATOR)
        .from_path(src)?;

    let mut metadata = Vec::new();

    for result in reader.records() {
        let row = result?;
        let group_name = &row[1];
        metadata.push(group_name.into())
    }

    Ok(metadata)
}

fn export<W>(
    writer: &mut W,
    sample_names: &[String],
    feature_names: &[String],
    counts: &[f64],
) -> io::Result<()>
where
    W: Write,
{
    for sample_name in sample_names {
        write!(writer, "\t{sample_name}")?;
    }

    writeln!(writer)?;

    let sample_count = sample_names.len();

    for (feature_name, row) in feature_names.iter().zip(counts.chunks_exact(sample_count)) {
        write!(writer, "{feature_name}")?;

        for n in row {
            write!(writer, "\t{n}")?;
        }

        writeln!(writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export() -> io::Result<()> {
        let mut buf = Vec::new();

        let sample_names = [String::from("s0"), String::from("s1")];
        let feature_names = [String::from("f0"), String::from("f1"), String::from("f2")];
        let counts = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        export(&mut buf, &sample_names, &feature_names, &counts)?;

        let expected = b"\ts0\ts1\nf0\t0\t1\nf1\t2\t3\nf2\t4\t5\n";
        assert_eq!(buf, expected);

        Ok(())
    }
}
