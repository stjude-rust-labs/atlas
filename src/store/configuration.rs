use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub struct Configuration {
    pub id: i32,
}

pub async fn find_or_create_configuration(
    tx: &mut Transaction<'_, Postgres>,
    annotations_id: i32,
    feature_type: &str,
    feature_name: &str,
) -> anyhow::Result<Configuration> {
    let configuration_id = sqlx::query_scalar!(
        "
        insert into configurations
            (annotation_id, feature_type, feature_name)
        values
            ($1, $2, $3)
        on conflict (annotation_id, feature_type, feature_name) do update
            set id = configurations.id
        returning id
        ",
        annotations_id,
        feature_type,
        feature_name,
    )
    .fetch_one(tx)
    .await?;

    Ok(Configuration {
        id: configuration_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::setup;

    #[tokio::test]
    async fn test_find_or_create_configuration() -> anyhow::Result<()> {
        use crate::store::annotations::find_or_create_annotations;

        let db = setup().await?;
        let mut tx = db.pool.begin().await?;

        let gencode_21 = find_or_create_annotations(&mut tx, "GENCODE 21", "GRCh38").await?;
        let gencode_40 = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;

        let configuration =
            find_or_create_configuration(&mut tx, gencode_40.id, "gene", "gene_name").await?;
        assert_eq!(configuration.id, 1);

        let configuration =
            find_or_create_configuration(&mut tx, gencode_40.id, "gene", "gene_name").await?;
        assert_eq!(configuration.id, 1);

        let configuration =
            find_or_create_configuration(&mut tx, gencode_40.id, "exon", "gene_id").await?;
        assert_eq!(configuration.id, 3);

        let configuration =
            find_or_create_configuration(&mut tx, gencode_21.id, "gene", "gene_name").await?;
        assert_eq!(configuration.id, 4);

        Ok(())
    }
}
