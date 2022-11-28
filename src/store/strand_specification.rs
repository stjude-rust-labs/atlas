use clap::ValueEnum;
use serde::Serialize;

#[derive(ValueEnum, Clone, Serialize, Copy, Debug, Eq, PartialEq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "strand_specification", rename_all = "lowercase")]
pub enum StrandSpecification {
    None,
    Forward,
    Reverse,
}
