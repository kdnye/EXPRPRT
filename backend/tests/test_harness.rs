use std::future::Future;

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn run_test<F, Fut>(test: F) -> Result<()>
where
    F: FnOnce(PgPool) -> Fut,
    Fut: Future<Output = Result<()>> + Send,
{
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("EXPENSES__DATABASE__URL"))
        .unwrap_or_else(|_| "postgres://expenses:expenses@localhost:5432/expenses".to_string());

    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Skipping integration test: unable to connect to database: {err}");
            return Ok(());
        }
    };

    sqlx::migrate!("./migrations").run(&pool).await?;

    test(pool).await
}
