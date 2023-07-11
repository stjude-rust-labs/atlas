use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub struct Annotations {
    pub id: i32,
}

pub async fn find_or_create_annotations(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    genome_build: &str,
) -> anyhow::Result<Annotations> {
    let annotations_id = sqlx::query_scalar!(
        "
        insert into annotations
            (name, genome_build)
        values
            ($1, $2)
        on conflict (name) do update
            set id = annotations.id
        returning id
        ",
        name,
        genome_build,
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(Annotations { id: annotations_id })
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_find_or_create_annotations(pool: PgPool) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;
        assert_eq!(annotations.id, 1);

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 40", "GRCh38.p13").await?;
        assert_eq!(annotations.id, 1);

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 39", "GRCh38.p13").await?;
        assert_eq!(annotations.id, 3);

        let annotations = find_or_create_annotations(&mut tx, "GENCODE 21", "GRCh38").await?;
        assert_eq!(annotations.id, 4);

        Ok(())
    }
}
