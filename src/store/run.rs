use sqlx::{Postgres, Transaction};

pub async fn run_exists(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    sample_id: i32,
) -> anyhow::Result<bool> {
    sqlx::query_scalar!(
        "
        select 1
        from runs
        where configuration_id = $1 and sample_id = $2
        limit 1
        ",
        configuration_id,
        sample_id,
    )
    .fetch_optional(tx)
    .await
    .map(|result| result.is_some())
    .map_err(|e| e.into())
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
) -> anyhow::Result<Run> {
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
    .fetch_one(tx)
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
    async fn test_run_exists(pool: PgPool) -> anyhow::Result<()> {
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

        assert!(run_exists(&mut tx, configuration.id, sample.id).await?);
        assert!(!run_exists(&mut tx, 1000, 1000).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_run(pool: PgPool) -> anyhow::Result<()> {
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
