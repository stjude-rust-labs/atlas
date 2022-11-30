use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::store::StrandSpecification;

use super::{types::Timestampz, Context, Error};

pub fn router() -> Router<Context> {
    Router::new()
        .route("/samples", get(index))
        .route("/samples/:name", get(show))
}

#[derive(Serialize)]
struct SamplesBody<T> {
    samples: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Sample {
    id: i32,
    name: String,
    created_at: Timestampz,
}

async fn index(State(ctx): State<Context>) -> super::Result<Json<SamplesBody<Vec<Sample>>>> {
    let samples = sqlx::query_as!(
        Sample,
        r#"select id, name, created_at "created_at: Timestampz" from samples"#
    )
    .fetch_all(&ctx.pool)
    .await?;

    Ok(Json(SamplesBody { samples }))
}

struct SampleFromQuery {
    name: String,
    counts_id: i32,
    counts_genome_build: String,
    counts_gene_model: String,
    counts_data_type: String,
    counts_feature_type: String,
    counts_feature_name: String,
    counts_strand_specification: StrandSpecification,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    id: i32,
    genome_build: String,
    gene_model: String,
    data_type: String,
    feature_type: String,
    feature_name: String,
    strand_specification: StrandSpecification,
}

#[derive(Serialize)]
struct SampleWithCounts {
    name: String,
    counts: Vec<Counts>,
}

async fn show(
    State(ctx): State<Context>,
    Path(name): Path<String>,
) -> super::Result<Json<SampleWithCounts>> {
    let rows = sqlx::query_as!(
        SampleFromQuery,
        r#"
            select
                samples.name,
                runs.id as counts_id,
                annotations.genome_build as counts_genome_build,
                annotations.name as counts_gene_model,
                configurations.feature_type as counts_feature_type,
                configurations.feature_name as counts_feature_name,
                configurations.strand_specification as "counts_strand_specification: _",
                runs.data_type as counts_data_type
            from samples
            inner join runs
                on runs.sample_id = samples.id
            inner join configurations
                on runs.configuration_id = configurations.id
            inner join annotations
                on configurations.annotation_id = annotations.id
            where samples.name = $1
        "#,
        name
    )
    .fetch_all(&ctx.pool)
    .await?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

    let first_row = rows.first().expect("missing first row");
    let name = first_row.name.clone();

    let counts = rows
        .into_iter()
        .map(|row| Counts {
            id: row.counts_id,
            genome_build: row.counts_genome_build,
            gene_model: row.counts_gene_model,
            data_type: row.counts_data_type,
            feature_type: row.counts_feature_type,
            feature_name: row.counts_feature_name,
            strand_specification: row.counts_strand_specification,
        })
        .collect();

    Ok(Json(SampleWithCounts { name, counts }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use sqlx::PgPool;
    use tower::ServiceExt;

    use super::*;

    fn app(pool: PgPool) -> Router {
        router().with_state(Context { pool })
    }

    #[sqlx::test(fixtures("samples"))]
    async fn test_index(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder().uri("/samples").body(Body::empty())?;
        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await?;
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "samples": [{
                    "id": 1,
                    "name": "sample_1",
                    "createdAt": "2022-02-18T21:05:05Z",
                }, {
                    "id": 2,
                    "name": "sample_2",
                    "createdAt": "2022-02-18T21:05:06Z",
                }]
            })
        );

        Ok(())
    }

    #[sqlx::test(fixtures("samples"))]
    async fn test_show(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/samples/sample_1")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;

        let body = hyper::body::to_bytes(response.into_body()).await?;
        let actual: Value = serde_json::from_slice(&body)?;

        assert_eq!(
            actual,
            json!({
                "name": "sample_1",
                "counts": [{
                    "id": 1,
                    "genomeBuild": "GRCh38.p13",
                    "geneModel": "GENCODE 39",
                    "featureType": "exon",
                    "featureName": "gene_name",
                    "strandSpecification": "reverse",
                    "dataType": "RNA-Seq",
                }, {
                    "id": 2,
                    "genomeBuild": "GRCh37.p13",
                    "geneModel": "GENCODE 19",
                    "featureType": "exon",
                    "featureName": "gene_name",
                    "strandSpecification": "reverse",
                    "dataType": "RNA-Seq",
                }],
            })
        );

        Ok(())
    }

    #[sqlx::test(fixtures("samples"))]
    async fn test_show_with_an_invalid_name(pool: PgPool) -> anyhow::Result<()> {
        let request = Request::builder()
            .uri("/samples/sample_x")
            .body(Body::empty())?;

        let response = app(pool).oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
