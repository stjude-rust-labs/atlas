use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub struct Sample {
    pub id: i32,
}

pub async fn find_or_create_sample(
    tx: &mut Transaction<'_, Postgres>,
    sample_name: &str,
) -> anyhow::Result<Sample> {
    let sample_id = sqlx::query_scalar!(
        "
        insert into samples (name) values ($1)
        on conflict (name) do update
            set id = samples.id
        returning id
        ",
        sample_name
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(Sample { id: sample_id })
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_find_or_create_sample(pool: PgPool) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

        let sample = find_or_create_sample(&mut tx, "sample1").await?;
        assert_eq!(sample.id, 1);

        let sample = find_or_create_sample(&mut tx, "sample1").await?;
        assert_eq!(sample.id, 1);

        let sample = find_or_create_sample(&mut tx, "sample2").await?;
        assert_eq!(sample.id, 3);

        Ok(())
    }
}
