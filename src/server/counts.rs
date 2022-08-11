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
            on runs.configuration_id = feature_names.configuration_id
        left join counts
            on runs.id = counts.run_id and counts.feature_name_id = feature_names.id
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
    use serde::Deserialize;
    use sqlx::PgPool;
    use tower::ServiceExt;

    use super::*;

    fn app(pool: PgPool) -> Router {
        router().layer(Extension(Context { pool }))
    }

    #[sqlx::test(fixtures("counts"))]
    async fn test_show(pool: PgPool) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct CountsBody {
            counts: HashMap<String, i32>,
        }

        let request = Request::builder().uri("/counts/1").body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await?;
        let actual: CountsBody = serde_json::from_slice(&body)?;

        let expected = [("feature_1".into(), 8), ("feature_2".into(), 0)]
            .into_iter()
            .collect();

        assert_eq!(actual.counts, expected);

        Ok(())
    }

    #[sqlx::test]
    async fn test_show_with_an_invalid_id(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder().uri("/counts/1597").body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
