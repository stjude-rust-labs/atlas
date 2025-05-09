use serde::Serialize;
use sqlx::PgExecutor;
use time::OffsetDateTime;

#[derive(Debug, Eq, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    id: i32,
    name: String,
    #[schema(inline)]
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

pub async fn all<'a, E>(executor: E) -> sqlx::Result<Vec<Sample>>
where
    E: PgExecutor<'a>,
{
    sqlx::query_as!(Sample, r#"select id, name, created_at from samples"#)
        .fetch_all(executor)
        .await
}

pub async fn find<'a, E>(executor: E, id: i32) -> sqlx::Result<Option<Sample>>
where
    E: PgExecutor<'a>,
{
    sqlx::query_as!(
        Sample,
        "select id, name, created_at from samples where id = $1",
        id
    )
    .fetch_optional(executor)
    .await
}

#[cfg(test)]
pub async fn find_or_create_sample(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sample_name: &str,
) -> sqlx::Result<i32> {
    sqlx::query_scalar!(
        "
        insert into samples (name) values ($1)
        on conflict (name) do update
            set id = samples.id
        returning id
        ",
        sample_name
    )
    .fetch_one(&mut **tx)
    .await
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
    use time::{Date, Month, Time};

    use super::*;

    #[sqlx::test(fixtures("sample_find"))]
    async fn test_find(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            find(&pool, 1).await?,
            Some(Sample {
                id: 1,
                name: String::from("sample_1"),
                created_at: OffsetDateTime::new_utc(
                    Date::from_calendar_date(2022, Month::February, 18)?,
                    Time::from_hms(21, 5, 5)?
                ),
            })
        );

        assert!(find(&pool, 3).await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_or_create_sample(pool: PgPool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        let sample_id = find_or_create_sample(&mut tx, "sample1").await?;
        assert_eq!(sample_id, 1);

        let sample_id = find_or_create_sample(&mut tx, "sample1").await?;
        assert_eq!(sample_id, 1);

        let sample_id = find_or_create_sample(&mut tx, "sample2").await?;
        assert_eq!(sample_id, 3);

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
