use std::{fs::File, io::Read, path::Path};

use atlas_core::counts::dimension_reduction::tsne;

use crate::cli;

pub fn run(args: cli::transform::tsne::Args) -> anyhow::Result<()> {
    let (labels, data, feature_count) = import(&args.src)?;

    let data = data.into_iter().map(|n| n as f64).collect();
    let embedding = tsne::transform(args.perplexity, args.theta, data, feature_count);

    for (label, point) in labels.iter().zip(embedding.chunks_exact(2)) {
        let (x, y) = (point[0], point[1]);
        println!("{label}\t{x}\t{y}");
    }

    Ok(())
}

fn import<P>(src: P) -> anyhow::Result<(Vec<String>, Vec<i32>, usize)>
where
    P: AsRef<Path>,
{
    let mut reader = File::open(src)?;
    import_inner(&mut reader)
}

fn import_inner<R>(reader: &mut R) -> anyhow::Result<(Vec<String>, Vec<i32>, usize)>
where
    R: Read,
{
    const SEPARATOR: u8 = b'\t';
    const META_PREFIX: &str = "__";

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(SEPARATOR)
        .from_reader(reader);

    let labels: Vec<_> = reader.headers()?.iter().skip(1).map(String::from).collect();
    let mut data = Vec::new();
    let mut feature_count = 0;

    for result in reader.records() {
        let row = result?;

        let mut fields = row.iter();

        let feature_name = fields
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing feature name"))?;

        if feature_name.starts_with(META_PREFIX) {
            break;
        }

        for field in fields {
            let n = field.parse()?;
            data.push(n);
        }

        feature_count += 1;
    }

    Ok((labels, data, feature_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_inner() -> anyhow::Result<()> {
        let src = b"\ts0\ts1\nf0\t0\t1\nf1\t2\t3\nf2\t4\t5\n";
        let (labels, data, feature_count) = import_inner(&mut &src[..])?;

        assert_eq!(labels, [String::from("s0"), String::from("s1")]);
        assert_eq!(data, [0, 1, 2, 3, 4, 5]);
        assert_eq!(feature_count, 3);

        Ok(())
    }
}
