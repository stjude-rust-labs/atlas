use std::{
    fs::File,
    io::{BufRead, BufReader},
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
    let _raw_corrected_counts =
        batch_effect::correct_raw(&counts, feature_count, sample_count, &metadata);

    todo!()
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
