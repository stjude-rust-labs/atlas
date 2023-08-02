use sqlx::{PgPool, Postgres, Transaction};

use super::StrandSpecification;

#[derive(Debug)]
pub struct Configuration {
    pub id: i32,
}

pub async fn exists(pool: &PgPool, id: i32) -> sqlx::Result<bool> {
    sqlx::query_scalar!(
        r#"select exists(select 1 from configurations where id = $1) as "exists!""#,
        id
    )
    .fetch_one(pool)
    .await
}

pub async fn find_or_create_configuration(
    tx: &mut Transaction<'_, Postgres>,
    annotations_id: i32,
    feature_type: &str,
    feature_name: &str,
    strand_specification: StrandSpecification,
) -> sqlx::Result<Configuration> {
    let configuration_id = sqlx::query_scalar!(
        "
        insert into configurations
            (annotation_id, feature_type, feature_name, strand_specification)
        values
            ($1, $2, $3, $4)
        on conflict (annotation_id, feature_type, feature_name) do update
            set id = configurations.id
        returning id
        ",
        annotations_id,
        feature_type,
        feature_name,
        strand_specification as _,
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(Configuration {
        id: configuration_id,
    })
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::store::annotations::find_or_create_annotations;

    #[sqlx::test]
    async fn test_exists(pool: PgPool) -> sqlx::Result<()> {
        assert!(!exists(&pool, 1).await?);

        let mut tx = pool.begin().await?;

        let gencode_40 = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;
        let configuration = find_or_create_configuration(
            &mut tx,
            gencode_40.id,
            "gene",
            "gene_name",
            StrandSpecification::Reverse,
        )
        .await?;

        assert!(!exists(&pool, configuration.id).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_or_create_configuration(pool: PgPool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        let gencode_21 = find_or_create_annotations(&mut tx, "GENCODE 21", "GRCh38").await?;
        let gencode_40 = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration = find_or_create_configuration(
            &mut tx,
            gencode_40.id,
            "gene",
            "gene_name",
            StrandSpecification::Reverse,
        )
        .await?;
        assert_eq!(configuration.id, 1);

        let configuration = find_or_create_configuration(
            &mut tx,
            gencode_40.id,
            "gene",
            "gene_name",
            StrandSpecification::Reverse,
        )
        .await?;
        assert_eq!(configuration.id, 1);

        let configuration = find_or_create_configuration(
            &mut tx,
            gencode_40.id,
            "exon",
            "gene_id",
            StrandSpecification::Reverse,
        )
        .await?;
        assert_eq!(configuration.id, 3);

        let configuration = find_or_create_configuration(
            &mut tx,
            gencode_21.id,
            "gene",
            "gene_name",
            StrandSpecification::Reverse,
        )
        .await?;
        assert_eq!(configuration.id, 4);

        Ok(())
    }
}
