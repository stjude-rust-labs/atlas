use std::collections::HashSet;

use futures::TryStreamExt;
use sqlx::{Postgres, Transaction};

pub async fn find_feature_names(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
) -> anyhow::Result<Vec<(i32, String)>> {
    let mut rows = sqlx::query!(
        "select id, name from feature_names where configuration_id = $1",
        configuration_id,
    )
    .fetch(&mut **tx);

    let mut names = Vec::new();

    while let Some(row) = rows.try_next().await? {
        names.push((row.id, row.name));
    }

    Ok(names)
}

pub async fn create_feature_names(
    tx: &mut Transaction<'_, Postgres>,
    configuration_id: i32,
    names: &HashSet<String>,
) -> anyhow::Result<Vec<(i32, String)>> {
    use std::iter;

    let configuration_ids: Vec<_> = iter::repeat(configuration_id).take(names.len()).collect();
    let names: Vec<_> = names.iter().cloned().collect();

    let mut rows = sqlx::query!(
        "
        insert into feature_names (configuration_id, name)
        select * from unnest($1::integer[], $2::text[])
        returning id, name
        ",
        &configuration_ids[..],
        &names[..]
    )
    .fetch(&mut **tx);

    let mut names = Vec::new();

    while let Some(row) = rows.try_next().await? {
        names.push((row.id, row.name));
    }

    Ok(names)
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::store::{
        annotations::find_or_create_annotations, configuration::find_or_create_configuration,
        StrandSpecification,
    };

    #[sqlx::test]
    async fn test_find_feature_names(pool: PgPool) -> anyhow::Result<()> {
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

        let feature_names = find_feature_names(&mut tx, configuration.id).await?;
        assert!(feature_names.is_empty());

        let names = [String::from("feature1"), String::from("feature2")]
            .into_iter()
            .collect();
        create_feature_names(&mut tx, configuration.id, &names).await?;

        let feature_names = find_feature_names(&mut tx, configuration.id).await?;
        assert_eq!(feature_names.len(), names.len());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_feature_names(pool: PgPool) -> anyhow::Result<()> {
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

        let names = [String::from("feature1"), String::from("feature2")]
            .into_iter()
            .collect();
        create_feature_names(&mut tx, configuration.id, &names).await?;

        let feature_names = find_feature_names(&mut tx, configuration.id).await?;
        assert_eq!(feature_names.len(), names.len());

        Ok(())
    }
}
