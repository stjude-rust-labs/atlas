use std::{collections::HashMap, io};

use sqlx::{Postgres, Transaction};

pub async fn create_counts(
    tx: &mut Transaction<'_, Postgres>,
    run_id: i32,
    features: &Vec<(i32, String)>,
    counts: &HashMap<String, u32>,
) -> anyhow::Result<()> {
    let mut run_ids = Vec::new();
    let mut feature_ids = Vec::new();
    let mut values = Vec::new();

    for (feature_id, name) in features {
        let count = counts
            .get(name)
            .copied()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;

        if count == 0 {
            continue;
        }

        run_ids.push(run_id);
        feature_ids.push(*feature_id);
        values.push(count as i64);
    }

    sqlx::query!(
        "
        insert into counts (run_id, feature_id, value)
        select * from unnest($1::integer[], $2::integer[], $3::bigint[])
        ",
        &run_ids[..],
        &feature_ids[..],
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
    use crate::store::feature::find_features;

    #[sqlx::test(fixtures("count_create_counts"))]
    async fn test_create_counts(pool: PgPool) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

        let configuration_id = 1;
        let run_id = 1;

        let features = find_features(&mut *tx, configuration_id).await?;

        let counts = [(String::from("feature1"), 8), (String::from("feature2"), 0)]
            .into_iter()
            .collect();

        create_counts(&mut tx, run_id, &features, &counts).await?;

        Ok(())
    }
}
