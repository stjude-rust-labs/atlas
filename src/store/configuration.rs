use serde::Serialize;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};

#[derive(Debug, Serialize, Eq, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AllResult {
    id: i32,
    annotation_name: String,
    annotation_genome_build: String,
    feature_type: String,
    feature_name: String,
}

pub async fn all<'a, E>(executor: E) -> sqlx::Result<Vec<AllResult>>
where
    E: PgExecutor<'a>,
{
    sqlx::query_as!(
        AllResult,
        r#"
        select
            configurations.id,
            annotations.name as annotation_name,
            annotations.genome_build as annotation_genome_build,
            configurations.feature_type,
            configurations.feature_name
        from configurations
        inner join annotations on configurations.annotation_id = annotations.id
        "#,
    )
    .fetch_all(executor)
    .await
}

pub async fn exists(pool: &PgPool, id: i32) -> sqlx::Result<bool> {
    sqlx::query_scalar!(
        r#"select exists(select 1 from configurations where id = $1) as "exists!""#,
        id
    )
    .fetch_one(pool)
    .await
}

#[derive(Debug)]
pub struct Configuration {
    pub id: i32,
}

pub async fn create_configuration(
    tx: &mut Transaction<'_, Postgres>,
    annotations_id: i32,
    feature_type: &str,
    feature_name: &str,
) -> sqlx::Result<Configuration> {
    let configuration_id = sqlx::query_scalar!(
        "
        insert into configurations
            (annotation_id, feature_type, feature_name)
        values
            ($1, $2, $3)
        returning id
        ",
        annotations_id,
        feature_type,
        feature_name,
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

    #[sqlx::test(fixtures("configuration_all"))]
    async fn test_all(pool: PgPool) -> sqlx::Result<()> {
        let configurations = all(&pool).await?;

        assert_eq!(
            configurations,
            [AllResult {
                id: 1,
                annotation_name: String::from("GENCODE 40"),
                annotation_genome_build: String::from("GRCh38.p13"),
                feature_type: String::from("gene"),
                feature_name: String::from("gene_name"),
            }]
        );

        Ok(())
    }

    #[sqlx::test(fixtures("configuration_exists"))]
    async fn test_exists(pool: PgPool) -> sqlx::Result<()> {
        assert!(exists(&pool, 1).await?);
        assert!(!exists(&pool, 2).await?);
        Ok(())
    }

    #[sqlx::test]
    async fn test_find_or_create_configuration(pool: PgPool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        let gencode_21 = find_or_create_annotations(&mut tx, "GENCODE 21", "GRCh38").await?;
        let gencode_40 = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration =
            create_configuration(&mut tx, gencode_40.id, "gene", "gene_name").await?;
        assert_eq!(configuration.id, 1);

        let configuration = create_configuration(&mut tx, gencode_40.id, "exon", "gene_id").await?;
        assert_eq!(configuration.id, 2);

        let configuration =
            create_configuration(&mut tx, gencode_21.id, "gene", "gene_name").await?;
        assert_eq!(configuration.id, 3);

        assert!(matches!(
            create_configuration(&mut tx, gencode_40.id, "gene", "gene_name").await,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation()
        ));

        Ok(())
    }
}
