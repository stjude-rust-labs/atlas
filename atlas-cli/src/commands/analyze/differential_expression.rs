use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
};

use atlas_core::counts::differential_expression::{self, FeatureResult};

use crate::cli;

const SEPARATOR: u8 = b'\t';

pub fn run(args: cli::analyze::differential_expression::Args) -> anyhow::Result<()> {
    let (sample_names, feature_names, counts) = import(&args.src)?;
    let groups = read_metadata(&args.metadata, &sample_names)?;

    let results = differential_expression::analyze(
        &counts,
        feature_names.len(),
        sample_names.len(),
        &groups,
        args.reference.as_deref(),
        args.target.as_deref(),
    )?;

    let stdout = io::stdout().lock();
    let mut writer = BufWriter::new(stdout);
    export(&mut writer, &feature_names, &results)?;

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

fn read_metadata<P>(src: P, sample_names: &[String]) -> anyhow::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let mut reader = File::open(src)?;
    read_metadata_inner(&mut reader, sample_names)
}

fn read_metadata_inner<R>(reader: &mut R, sample_names: &[String]) -> anyhow::Result<Vec<String>>
where
    R: Read,
{
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(SEPARATOR)
        .from_reader(reader);

    let mut groups_by_sample = HashMap::new();

    for result in reader.records() {
        let row = result?;

        let sample_name = row
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("missing sample name in metadata"))?;
        let group_name = row
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("missing condition for sample {sample_name}"))?;

        groups_by_sample.insert(sample_name.to_owned(), group_name.to_owned());
    }

    sample_names
        .iter()
        .map(|name| {
            groups_by_sample
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing metadata for sample {name}"))
        })
        .collect()
}

fn export<W>(writer: &mut W, feature_names: &[String], results: &[FeatureResult]) -> io::Result<()>
where
    W: Write,
{
    writeln!(writer, "\tbaseMean\tlog2FoldChange\tlfcSE\tstat\tpvalue")?;

    for (feature_name, r) in feature_names.iter().zip(results) {
        writeln!(
            writer,
            "{feature_name}\t{}\t{}\t{}\t{}\t{}",
            r.base_mean, r.log2_fold_change, r.lfc_se, r.stat, r.pvalue
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_inner() -> anyhow::Result<()> {
        let src = b"\ts0\ts1\nf0\t3\t5\nf1\t8\t13\n";

        let (sample_names, feature_names, counts) = import_inner(&mut &src[..])?;

        assert_eq!(sample_names, [String::from("s0"), String::from("s1")]);
        assert_eq!(feature_names, [String::from("f0"), String::from("f1")]);
        assert_eq!(counts, [3, 5, 8, 13]);

        Ok(())
    }

    #[test]
    fn test_read_metadata_inner() -> anyhow::Result<()> {
        let meta = b"\tcondition\ns1\tB\ns0\tA\n";
        let samples = [String::from("s0"), String::from("s1")];

        let groups = read_metadata_inner(&mut &meta[..], &samples)?;
        assert_eq!(groups, [String::from("A"), String::from("B")]);

        let missing = [String::from("s0"), String::from("s2")];
        assert!(read_metadata_inner(&mut &meta[..], &missing).is_err());

        Ok(())
    }

    #[test]
    fn test_export() -> io::Result<()> {
        let mut buf = Vec::new();

        let feature_names = [String::from("gene0"), String::from("gene1")];

        let results = [
            FeatureResult {
                base_mean: 100.0,
                log2_fold_change: 1.5,
                lfc_se: 0.25,
                stat: 6.0,
                pvalue: 0.001,
            },
            FeatureResult {
                base_mean: 0.0,
                log2_fold_change: f64::NAN,
                lfc_se: f64::NAN,
                stat: f64::NAN,
                pvalue: f64::NAN,
            },
        ];

        export(&mut buf, &feature_names, &results)?;

        let expected = b"\tbaseMean\tlog2FoldChange\tlfcSE\tstat\tpvalue\ngene0\t100\t1.5\t0.25\t6\t0.001\ngene1\t0\tNaN\tNaN\tNaN\tNaN\n";
        assert_eq!(buf, expected);

        Ok(())
    }
}
