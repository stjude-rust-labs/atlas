use std::{collections::HashMap, num::ParseIntError};

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::server::Error;

use super::Context;

#[derive(Debug, Deserialize)]
struct IndexQuery {
    run_ids: String,
}

#[derive(Serialize)]
struct Run {
    id: i32,
    counts: HashMap<String, i32>,
}

#[derive(Serialize)]
struct IndexBody {
    runs: Vec<Run>,
}

pub fn router() -> Router<Context> {
    Router::new().route("/counts", get(index))
}

async fn index(
    State(ctx): State<Context>,
    Query(params): Query<IndexQuery>,
) -> super::Result<Json<IndexBody>> {
    const DELIMITER: char = ',';

    let run_ids: Vec<i32> = params
        .run_ids
        .split(DELIMITER)
        .map(|s| {
            s.parse()
                .map_err(|e: ParseIntError| super::Error::Anyhow(e.into()))
        })
        .collect::<Result<_, _>>()?;

    if run_ids.is_empty() {
        return Err(Error::NotFound);
    }

    let feature_names: Vec<_> = sqlx::query!(
        r#"
        select
            features.name
        from features
        inner join configurations
            on features.configuration_id = configurations.id
        inner join runs
            on configurations.id = runs.configuration_id
        where runs.id = $1
        "#,
        // SAFETY: `run_ids` is non-empty.
        run_ids[0]
    )
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .map(|record| record.name)
    .collect();

    let counts: Vec<_> = sqlx::query!(
        r#"
        select
            coalesce(counts.value, 0) as "value!"
        from runs
        inner join configurations
            on runs.configuration_id = configurations.id
        inner join features
            on runs.configuration_id = features.configuration_id
        left join counts
            on runs.id = counts.run_id and counts.feature_id = features.id
        where runs.id in (select unnest($1::integer[]))
        "#,
        &run_ids[..]
    )
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .map(|record| record.value)
    .collect();

    if counts.is_empty() {
        return Err(Error::NotFound);
    }

    let mut runs = Vec::with_capacity(run_ids.len());
    let chunks = counts.chunks_exact(feature_names.len());

    for (id, chunk) in run_ids.into_iter().zip(chunks) {
        let counts = feature_names
            .iter()
            .zip(chunk)
            .map(|(name, count)| (name.clone(), *count))
            .collect();

        runs.push(Run { id, counts });
    }

    Ok(Json(IndexBody { runs }))
}
