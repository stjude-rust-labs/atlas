use clap::ValueEnum;
use serde::Serialize;

#[derive(ValueEnum, Clone, Serialize, Copy, Debug, Eq, PartialEq, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "strand_specification", rename_all = "lowercase")]
pub enum StrandSpecification {
    None,
    Forward,
    Reverse,
}

impl From<StrandSpecification> for atlas_core::StrandSpecification {
    fn from(strand_specification: StrandSpecification) -> Self {
        match strand_specification {
            StrandSpecification::None => Self::None,
            StrandSpecification::Forward => Self::Forward,
            StrandSpecification::Reverse => Self::Reverse,
        }
    }
}
