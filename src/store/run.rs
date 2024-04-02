use sqlx::{PgExecutor, Postgres, Transaction};

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

#[derive(Debug)]
pub struct Run {
    pub id: i32,
}

pub async fn create_run(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    sample_id: i32,
    data_type: &str,
) -> sqlx::Result<Run> {
    let run_id = sqlx::query_scalar!(
        "
        insert into runs
            (sample_id, configuration_id, data_type)
        values
            ($1, $2, $3)
        returning id
        ",
        sample_id,
        configuration_id,
        data_type,
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(Run { id: run_id })
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::store::{
        annotations::find_or_create_annotations, configuration::find_or_create_configuration,
        sample::find_or_create_sample, StrandSpecification,
    };

    #[sqlx::test]
    async fn test_runs_exists(pool: PgPool) -> sqlx::Result<()> {
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
        create_run(&mut tx, configuration.id, sample.id, "RNA-Seq").await?;

        let sample_ids = [sample.id];
        assert!(runs_exists(&mut *tx, configuration.id, &sample_ids).await?);

        let sample_ids = [1000];
        assert!(!runs_exists(&mut *tx, 1000, &sample_ids).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_run(pool: PgPool) -> sqlx::Result<()> {
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
        assert_eq!(run.id, 1);

        Ok(())
    }
}
