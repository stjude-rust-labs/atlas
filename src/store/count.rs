use std::{collections::HashMap, io};

use sqlx::{Postgres, Transaction};

pub async fn create_counts(
    tx: &mut Transaction<'_, Postgres>,
    run_id: i32,
    feature_names: &Vec<(i32, String)>,
    counts: &HashMap<String, u64>,
) -> anyhow::Result<()> {
    let mut run_ids = Vec::new();
    let mut feature_name_ids = Vec::new();
    let mut values = Vec::new();

    for (feature_name_id, name) in feature_names {
        let count = counts
            .get(name)
            .copied()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;

        if count == 0 {
            continue;
        }

        run_ids.push(run_id);
        feature_name_ids.push(*feature_name_id);
        values.push(count as i64);
    }

    sqlx::query!(
        "
        insert into counts (run_id, feature_name_id, value)
        select * from unnest($1::integer[], $2::integer[], $3::bigint[])
        ",
        &run_ids[..],
        &feature_name_ids[..],
        &values[..],
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::store::{
        annotations::find_or_create_annotations,
        configuration::find_or_create_configuration,
        feature_name::{create_feature_names, find_feature_names},
        run::create_run,
        sample::find_or_create_sample,
        StrandSpecification,
    };

    #[sqlx::test]
    async fn test_create_counts(pool: PgPool) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration = find_or_create_configuration(
            &mut tx,
            annotations.id,
            "gene",
            "gene_name",
            StrandSpecification::Reverse,
        )
        .await?;

        let sample = find_or_create_sample(&mut tx, "sample1").await?;
        let run = create_run(&mut tx, configuration.id, sample.id, "RNA-Seq").await?;

        let names = [String::from("feature1"), String::from("feature2")]
            .into_iter()
            .collect();
        create_feature_names(&mut tx, configuration.id, &names).await?;

        let feature_names = find_feature_names(&mut tx, configuration.id).await?;
        let counts = [(String::from("feature1"), 8), (String::from("feature2"), 0)]
            .into_iter()
            .collect();
        create_counts(&mut tx, run.id, &feature_names, &counts).await?;

        Ok(())
    }
}
