use clap::ValueEnum;
use serde::Serialize;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};

#[derive(ValueEnum, Clone, Serialize, Copy, Debug, Eq, PartialEq, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "strand_specification", rename_all = "lowercase")]
pub enum StrandSpecification {
    None,
    Forward,
    Reverse,
}

impl PgHasArrayType for StrandSpecification {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_strand_specification")
    }
}
