use clap::ArgEnum;
use serde::Serialize;

#[derive(ArgEnum, Clone, Serialize, Copy, Debug, Eq, PartialEq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "strand_specification", rename_all = "lowercase")]
pub enum StrandSpecification {
    None,
    Forward,
    Reverse,
}
