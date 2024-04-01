use sqlx::PgExecutor;

#[derive(Debug)]
pub struct Sample {
    pub id: i32,
}

#[cfg(test)]
pub async fn find_or_create_sample(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sample_name: &str,
) -> sqlx::Result<Sample> {
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

pub async fn find_or_create_samples<'a, E>(
    executor: E,
    sample_names: &[String],
) -> sqlx::Result<Vec<i32>>
where
    E: PgExecutor<'a>,
{
    let records = sqlx::query!(
        "
        insert into samples (name)
        select * from unnest($1::text[])
        on conflict (name)
            do update set id = samples.id
        returning id
        ",
        sample_names,
    )
    .fetch_all(executor)
    .await?;

    Ok(records.into_iter().map(|record| record.id).collect())
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_find_or_create_sample(pool: PgPool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        let sample = find_or_create_sample(&mut tx, "sample1").await?;
        assert_eq!(sample.id, 1);

        let sample = find_or_create_sample(&mut tx, "sample1").await?;
        assert_eq!(sample.id, 1);

        let sample = find_or_create_sample(&mut tx, "sample2").await?;
        assert_eq!(sample.id, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_or_create_samples(pool: PgPool) -> sqlx::Result<()> {
        let sample_names = [String::from("sample1")];
        let sample_ids = find_or_create_samples(&pool, &sample_names).await?;
        assert_eq!(sample_ids, [1]);

        let sample_names = [String::from("sample1")];
        let sample_ids = find_or_create_samples(&pool, &sample_names).await?;
        assert_eq!(sample_ids, [1]);

        let sample_names = [String::from("sample1"), String::from("sample2")];
        let sample_ids = find_or_create_samples(&pool, &sample_names).await?;
        assert_eq!(sample_ids, [1, 4]);

        Ok(())
    }
}
