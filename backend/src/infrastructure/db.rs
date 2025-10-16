use anyhow::Context;
use sqlx::postgres::PgPoolOptions;

use super::config::DatabaseConfig;

pub type PgPool = sqlx::Pool<sqlx::Postgres>;

pub async fn connect(config: &DatabaseConfig) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await
        .with_context(|| "failed to connect to PostgreSQL")
}
