use sqlx::PgExecutor;

pub async fn create<'a, E>(executor: E, name: &str) -> sqlx::Result<i32>
where
    E: PgExecutor<'a>,
{
    sqlx::query_scalar!("insert into datasets (name) values ($1) returning id", name)
        .fetch_one(executor)
        .await
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_create(pool: PgPool) -> sqlx::Result<()> {
        let id = create(&pool, "dataset_1").await?;
        assert_eq!(id, 1);

        assert!(matches!(
            create(&pool, "dataset_1").await,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation()
        ));

        Ok(())
    }
}
