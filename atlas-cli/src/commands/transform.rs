use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::bail;
use atlas_core::counts::dimension_reduction::tsne;

use crate::cli::transform;

pub fn transform(args: transform::Args) -> anyhow::Result<()> {
    let (labels, data, feature_count) = import(&args.srcs)?;

    let embedding = tsne::transform(args.perplexity, args.theta, data, feature_count);

    for (label, point) in labels.iter().zip(embedding.chunks_exact(2)) {
        let (x, y) = (point[0], point[1]);
        println!("{label}\t{x}\t{y}");
    }

    Ok(())
}

fn import<P>(srcs: &[P]) -> anyhow::Result<(Vec<String>, Vec<i32>, usize)>
where
    P: AsRef<Path>,
{
    const LINE_FEED: char = '\n';
    const CARRIAGE_RETURN: char = '\r';

    const SEPARATOR: char = '\t';
    const META_PREFIX: &str = "__";

    let mut labels = Vec::new();
    let mut data = Vec::new();
    let mut feature_count = 0;

    let mut line = String::new();

    for (i, src) in srcs.iter().enumerate() {
        let src = src.as_ref();

        match src.file_name().and_then(|s| s.to_str()) {
            Some(filename) => match filename.split_once('.') {
                Some((sample_name, _)) => labels.push(sample_name.into()),
                None => bail!("invalid filename"),
            },
            None => bail!("invalid filename"),
        }

        let mut reader = File::open(src).map(BufReader::new)?;

        line.clear();

        while reader.read_line(&mut line)? != 0 {
            if line.ends_with(LINE_FEED) {
                line.pop();

                if line.ends_with(CARRIAGE_RETURN) {
                    line.pop();
                }
            }

            let Some((name, raw_count)) = line.split_once(SEPARATOR) else {
                bail!("invalid row");
            };

            if name.starts_with(META_PREFIX) {
                break;
            }

            let n = raw_count.parse()?;

            data.push(n);

            line.clear();
        }

        if i == 0 {
            feature_count = data.len();
        }
    }

    Ok((labels, data, feature_count))
}
