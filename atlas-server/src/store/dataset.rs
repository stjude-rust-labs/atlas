use std::iter;

use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use sqlx::{PgExecutor, postgres::PgQueryResult};

#[derive(Serialize)]
pub struct Dataset {
    id: i32,
    name: String,
}

pub async fn all<'a, E>(executor: E) -> sqlx::Result<Vec<Dataset>>
where
    E: PgExecutor<'a>,
{
    sqlx::query_as!(Dataset, "select id, name from datasets")
        .fetch_all(executor)
        .await
}

pub async fn find<'a, E>(executor: E, id: i32) -> sqlx::Result<Option<Dataset>>
where
    E: PgExecutor<'a>,
{
    sqlx::query_as!(Dataset, "select id, name from datasets where id = $1", id)
        .fetch_optional(executor)
        .await
}

pub async fn create<'a, E>(executor: E, name: &str) -> sqlx::Result<i32>
where
    E: PgExecutor<'a>,
{
    sqlx::query_scalar!("insert into datasets (name) values ($1) returning id", name)
        .fetch_one(executor)
        .await
}

pub async fn create_runs<'a, E>(executor: E, dataset_id: i32, run_ids: &[i32]) -> sqlx::Result<()>
where
    E: PgExecutor<'a>,
{
    let dataset_ids: Vec<_> = iter::repeat(dataset_id).take(run_ids.len()).collect();

    sqlx::query!(
        "
        insert into datasets_runs (dataset_id, run_id)
        select * from unnest($1::integer[], $2::integer[])
        ",
        &dataset_ids[..],
        run_ids,
    )
    .execute(executor)
    .await
    .map(|_| ())
}

pub async fn exists<'a, E>(executor: E, id: i32) -> sqlx::Result<bool>
where
    E: PgExecutor<'a>,
{
    sqlx::query_scalar!(
        r#"select exists(select 1 from datasets where id = $1) as "exists!""#,
        id
    )
    .fetch_one(executor)
    .await
}

pub async fn add<'a, E>(executor: E, dataset_id: i32, run_id: i32) -> sqlx::Result<PgQueryResult>
where
    E: PgExecutor<'a>,
{
    sqlx::query!(
        "insert into datasets_runs (dataset_id, run_id) values ($1, $2)",
        dataset_id,
        run_id
    )
    .execute(executor)
    .await
}

pub async fn configuration_ids<'a, E>(executor: E, id: i32) -> sqlx::Result<Vec<i32>>
where
    E: PgExecutor<'a>,
{
    sqlx::query!(
        "
        select distinct(configuration_id)
        from datasets_runs
        inner join runs
            on datasets_runs.run_id = runs.id
        where datasets_runs.dataset_id = $1
        ",
        id
    )
    .fetch(executor)
    .map(|result| result.map(|row| row.configuration_id))
    .try_collect()
    .await
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_create(pool: PgPool) -> sqlx::Result<()> {
        let id = create(&pool, "dataset_1").await?;
        assert_eq!(id, 1);

        assert!(matches!(
            create(&pool, "dataset_1").await,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation()
        ));

        Ok(())
    }

    #[sqlx::test(fixtures("dataset_exists"))]
    async fn test_exists(pool: PgPool) -> sqlx::Result<()> {
        assert!(exists(&pool, 1).await?);
        assert!(!exists(&pool, 2).await?);
        Ok(())
    }
}
