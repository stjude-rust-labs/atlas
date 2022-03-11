use std::collections::HashMap;

use axum::{
    extract::{Extension, Path},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use super::{Context, Error};

pub fn router() -> Router {
    Router::new().route("/counts/:id", get(show))
}

#[derive(Serialize)]
struct CountsBody {
    counts: HashMap<String, i32>,
}

struct Count {
    name: String,
    value: i32,
}

async fn show(ctx: Extension<Context>, Path(id): Path<i32>) -> super::Result<Json<CountsBody>> {
    let rows = sqlx::query_as!(
        Count,
        r#"
        select
            feature_names.name,
            coalesce(counts.value, 0) as "value!"
        from runs
        inner join configurations
            on runs.configuration_id = configurations.id
        inner join feature_names
            on feature_names.configuration_id  = configurations.id
        left join counts
            on counts.feature_name_id = feature_names.id
        where runs.id = $1
        "#,
        id
    )
    .fetch_all(&ctx.pool)
    .await?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

    let counts = rows.into_iter().map(|c| (c.name, c.value)).collect();

    Ok(Json(CountsBody { counts }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use clap::Parser;
    use serde::Deserialize;
    use sqlx::PgPool;
    use tower::ServiceExt;

    use crate::{cli::ServerConfig, server::tests::TestPgDatabase};

    use super::*;

    async fn seed(pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!("insert into samples (name) values ('sample_1'), ('sample_2')")
            .execute(pool)
            .await?;

        sqlx::query!(
            "
            insert into annotations
                (name, genome_build)
            values
                ('GENCODE 39', 'GRCh38.p13'),
                ('GENCODE 19', 'GRCh37.p13')
            ",
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "
            insert into configurations
                (annotation_id, feature_type, feature_name)
            values
                (1, 'exon', 'gene_name'),
                (2, 'exon', 'gene_name');
            ",
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "
            insert into feature_names
                (configuration_id, name)
            values
                (1, 'feature_1'),
                (1, 'feature_2'),
                (2, 'feature_1'),
                (2, 'feature_2')
            ",
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "
            insert into runs
                (sample_id, configuration_id, data_type)
            values
                (1, 1, 'RNA-Seq'),
                (1, 2, 'RNA-Seq'),
                (2, 1, 'RNA-Seq')
            "
        )
        .execute(pool)
        .await?;

        sqlx::query!("insert into counts (run_id, feature_name_id, value) values (1, 1, 8)")
            .execute(pool)
            .await?;

        Ok(())
    }

    async fn setup() -> anyhow::Result<TestPgDatabase> {
        dotenv::dotenv().ok();

        let config = ServerConfig::parse();
        let db = TestPgDatabase::new(&config.database_url).await?;

        seed(&db.pool).await?;

        Ok(db)
    }

    fn app(db: &TestPgDatabase) -> Router {
        router().layer(Extension(Context {
            pool: db.pool.clone(),
        }))
    }

    #[tokio::test]
    async fn test_show() -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct CountsBody {
            counts: HashMap<String, i32>,
        }

        let db = setup().await?;

        let request = Request::builder().uri("/counts/1").body(Body::empty())?;
        let response = app(&db).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await?;
        let actual: CountsBody = serde_json::from_slice(&body)?;

        let expected = [("feature_1".into(), 8), ("feature_2".into(), 0)]
            .into_iter()
            .collect();

        assert_eq!(actual.counts, expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_show_with_an_invalid_id() -> anyhow::Result<()> {
        let db = setup().await?;

        let request = Request::builder().uri("/counts/1597").body(Body::empty())?;
        let response = app(&db).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
