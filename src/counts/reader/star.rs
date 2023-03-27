use std::collections::HashMap;

use tokio::io::AsyncBufRead;

use super::read_line;
use crate::store::StrandSpecification;

#[allow(dead_code)]
pub async fn read_counts<R>(
    feature_name: &str,
    strand_specification: StrandSpecification,
    reader: &mut R,
) -> anyhow::Result<HashMap<String, u64>>
where
    R: AsyncBufRead + Unpin,
{
    const COMMENT_PREFIX: char = '#';
    const COLUMN_COUNT: usize = 9;
    const DELIMITER: char = '\t';
    const META_LINE_COUNT: usize = 6;

    let name_index = match feature_name {
        "gene_id" => 0,
        "gene_name" => 1,
        _ => anyhow::bail!("invalid feature name: {}", feature_name),
    };

    let count_index = match strand_specification {
        StrandSpecification::None => 3,
        StrandSpecification::Forward => 4,
        StrandSpecification::Reverse => 5,
    };

    // SAFETY: `count_index` is at minimum 3.
    let count_offset = count_index - name_index - 1;

    let mut line = String::new();
    let mut counts = HashMap::new();

    for _ in 0..META_LINE_COUNT {
        line.clear();
        read_line(reader, &mut line).await?;
    }

    loop {
        line.clear();

        if read_line(reader, &mut line).await? == 0 {
            break;
        }

        let mut row = line.splitn(COLUMN_COUNT, DELIMITER);

        let raw_name = row
            .nth(name_index)
            .ok_or_else(|| anyhow::anyhow!("missing name in column {name_index}"))?;

        let raw_count = row
            .nth(count_offset)
            .ok_or_else(|| anyhow::anyhow!("missing count in column {count_offset}"))?;
        let count = raw_count.parse()?;

        counts.insert(raw_name.into(), count);
    }

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_counts() -> anyhow::Result<()> {
        let data = b"\
# gene-model: GENCODE v43
gene_id\tgene_name\tgene_type\tunstranded\tstranded_first\tstranded_second\ttpm_unstranded\tfpkm_unstranded\tfpkm_uq_unstranded
N_unmapped\t\t\t0\t0\t0\t\t\t
N_multimapping\t\t\t0\t0\t0\t\t\t
N_noFeature\t\t\t0\t0\t0\t\t\t
N_ambiguous\t\t\t0\t0\t0\t\t\t
ATLAS1.1\tfeature_1\tprotein_coding\t21\t13\t8\t0.0\t0.0\t0.0
ATLAS2.1\tfeature_2\tprotein_coding\t89\t55\t34\t0.0\t0.0\t0.0
";

        let mut reader = &data[..];
        let counts = read_counts("gene_name", StrandSpecification::None, &mut reader).await?;
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 21);
        assert_eq!(counts["feature_2"], 89);

        let mut reader = &data[..];
        let counts = read_counts("gene_name", StrandSpecification::Forward, &mut reader).await?;
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["feature_1"], 13);
        assert_eq!(counts["feature_2"], 55);

        let mut reader = &data[..];
        let counts = read_counts("gene_id", StrandSpecification::Reverse, &mut reader).await?;
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["ATLAS1.1"], 8);
        assert_eq!(counts["ATLAS2.1"], 34);

        Ok(())
    }
}
