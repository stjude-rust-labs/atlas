use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};
use sqlx::{PgExecutor, Postgres, Transaction};

pub async fn count<'a, E>(executor: E, configuration_id: i32) -> sqlx::Result<i64>
where
    E: PgExecutor<'a>,
{
    sqlx::query!(
        r#"select count(*) as "count!" from features where configuration_id = $1"#,
        configuration_id
    )
    .fetch_one(executor)
    .await
    .map(|record| record.count)
}

pub async fn find_features<'a, E>(
    executor: E,
    configuration_id: i32,
) -> sqlx::Result<Vec<(i32, String)>>
where
    E: PgExecutor<'a>,
{
    let mut rows = sqlx::query!(
        "select id, name from features where configuration_id = $1",
        configuration_id,
    )
    .fetch(executor);

    let mut names = Vec::new();

    while let Some(row) = rows.try_next().await? {
        names.push((row.id, row.name));
    }

    Ok(names)
}

pub async fn create_features(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    names: &[String],
    lengths: &[i32],
) -> sqlx::Result<Vec<(i32, String)>> {
    use std::iter;

    let configuration_ids: Vec<_> = iter::repeat(configuration_id).take(names.len()).collect();

    let mut rows = sqlx::query!(
        "
        insert into features (configuration_id, name, length)
        select * from unnest($1::integer[], $2::text[], $3::integer[])
        returning id, name
        ",
        &configuration_ids[..],
        names,
        lengths,
    )
    .fetch(&mut **tx);

    let mut names = Vec::new();

    while let Some(row) = rows.try_next().await? {
        names.push((row.id, row.name));
    }

    Ok(names)
}

pub async fn find_lengths_by_run_id<'a, E>(
    executor: E,
    run_id: i32,
) -> sqlx::Result<HashMap<String, i32>>
where
    E: PgExecutor<'a>,
{
    sqlx::query!(
        "
        select features.name, features.length
        from runs
        inner join features
            on runs.configuration_id = features.configuration_id
        where runs.id = $1
        ",
        run_id
    )
    .fetch(executor)
    .map(|result| result.map(|row| (row.name, row.length)))
    .try_collect()
    .await
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::store::{
        annotations::find_or_create_annotations, configuration::create_configuration,
    };

    #[sqlx::test]
    async fn test_count(pool: PgPool) -> sqlx::Result<()> {
        assert_eq!(count(&pool, 1).await?, 0);

        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration =
            create_configuration(&mut tx, annotations.id, "gene", "gene_name").await?;

        let names = [String::from("feature1"), String::from("feature2")];
        let lengths = [8, 13];
        create_features(&mut tx, configuration.id, &names, &lengths).await?;

        tx.commit().await?;

        assert_eq!(count(&pool, configuration.id).await?, 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_features(pool: PgPool) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration =
            create_configuration(&mut tx, annotations.id, "gene", "gene_name").await?;

        let features = find_features(&mut *tx, configuration.id).await?;
        assert!(features.is_empty());

        let names = [String::from("feature1"), String::from("feature2")];
        let lengths = [8, 13];
        create_features(&mut tx, configuration.id, &names, &lengths).await?;

        let features = find_features(&mut *tx, configuration.id).await?;
        assert_eq!(features.len(), names.len());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_features(pool: PgPool) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration =
            create_configuration(&mut tx, annotations.id, "gene", "gene_name").await?;

        let names = [String::from("feature1"), String::from("feature2")];
        let lengths = [8, 13];
        create_features(&mut tx, configuration.id, &names, &lengths).await?;

        let features = find_features(&mut *tx, configuration.id).await?;
        assert_eq!(features.len(), names.len());

        Ok(())
    }
}
