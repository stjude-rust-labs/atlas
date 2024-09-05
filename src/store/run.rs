use serde::Serialize;
use sqlx::{PgExecutor, Postgres, Transaction};

use super::StrandSpecification;

#[derive(Serialize)]
pub struct Run {
    id: i32,
    sample_id: i32,
    configuration_id: i32,
    strand_specification: StrandSpecification,
    data_type: String,
}

pub async fn where_sample_id<'a, E>(executor: E, id: i32) -> sqlx::Result<Vec<Run>>
where
    E: PgExecutor<'a>,
{
    sqlx::query_as!(
        Run,
        r#"
        select
            id,
            sample_id,
            configuration_id,
            strand_specification as "strand_specification: _",
            data_type
        from runs
        where id = $1
        "#,
        id
    )
    .fetch_all(executor)
    .await
}

pub async fn runs_exists<'a, E>(
    executor: E,
    configuration_id: i32,
    sample_ids: &[i32],
) -> sqlx::Result<bool>
where
    E: PgExecutor<'a>,
{
    sqlx::query_scalar!(
        r#"
        select count(*) as "count!"
        from runs
        where configuration_id = $1
            and sample_id in (select unnest($2::integer[]))
        "#,
        configuration_id,
        sample_ids,
    )
    .fetch_one(executor)
    .await
    .map(|n| n > 0)
}

#[cfg(test)]
pub async fn create_run(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    sample_id: i32,
    strand_specification: StrandSpecification,
    data_type: &str,
) -> sqlx::Result<i32> {
    sqlx::query_scalar!(
        "
        insert into runs
            (sample_id, configuration_id, strand_specification, data_type)
        values
            ($1, $2, $3, $4)
        returning id
        ",
        sample_id,
        configuration_id,
        strand_specification as _,
        data_type,
    )
    .fetch_one(&mut **tx)
    .await
}

pub async fn create_runs(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    sample_ids: &[i32],
    strand_specification: StrandSpecification,
    data_type: &str,
) -> sqlx::Result<Vec<i32>> {
    use std::iter;

    let sample_count = sample_ids.len();
    let configuration_ids: Vec<_> = iter::repeat(configuration_id).take(sample_count).collect();
    let strand_specifications: Vec<_> = iter::repeat(strand_specification)
        .take(sample_count)
        .collect();
    let data_types: Vec<_> = iter::repeat(data_type)
        .map(String::from)
        .take(sample_count)
        .collect();

    let records = sqlx::query!(
        "
        insert into runs (sample_id, configuration_id, strand_specification, data_type)
        select * from unnest($1::integer[], $2::integer[], $3::strand_specification[], $4::text[])
        returning id
        ",
        sample_ids,
        &configuration_ids[..],
        &strand_specifications[..] as _,
        &data_types[..],
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(records.into_iter().map(|record| record.id).collect())
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::store::{
        annotations::find_or_create_annotations, configuration, sample::find_or_create_sample,
    };

    #[sqlx::test]
    async fn test_runs_exists(pool: PgPool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration_id =
            configuration::create(&mut tx, annotations.id, "gene", "gene_name").await?;

        let sample_id = find_or_create_sample(&mut tx, "sample1").await?;
        create_run(
            &mut tx,
            configuration_id,
            sample_id,
            StrandSpecification::Reverse,
            "RNA-Seq",
        )
        .await?;

        let sample_ids = [sample_id];
        assert!(runs_exists(&mut *tx, configuration_id, &sample_ids).await?);

        let sample_ids = [1000];
        assert!(!runs_exists(&mut *tx, 1000, &sample_ids).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_runs(pool: PgPool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration_id =
            configuration::create(&mut tx, annotations.id, "gene", "gene_name").await?;

        let sample_id = find_or_create_sample(&mut tx, "sample1").await?;
        let sample_ids = [sample_id];

        let run_ids = create_runs(
            &mut tx,
            configuration_id,
            &sample_ids,
            StrandSpecification::Reverse,
            "RNA-Seq",
        )
        .await?;

        assert_eq!(run_ids, [1]);

        Ok(())
    }
}
