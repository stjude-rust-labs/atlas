use clap::ArgEnum;

#[derive(ArgEnum, Clone, Copy, Debug, Eq, PartialEq, sqlx::Type)]
#[sqlx(type_name = "strand_specification", rename_all = "lowercase")]
pub enum StrandSpecification {
    None,
    Forward,
    Reverse,
}
