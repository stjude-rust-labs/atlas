pub mod annotations;
pub mod configuration;
pub mod count;
pub mod feature_name;
pub mod run;
pub mod sample;

#[cfg(test)]
pub mod tests {
    use std::env;

    use crate::server::tests::TestPgDatabase;

    pub async fn setup() -> anyhow::Result<TestPgDatabase> {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("missing DATABASE_URL");
        let db = TestPgDatabase::new(&database_url).await?;

        Ok(db)
    }
}
