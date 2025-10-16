use anyhow::Context;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;

use super::config::DatabaseConfig;

pub type PgPool = sqlx::Pool<sqlx::Postgres>;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn connect(config: &DatabaseConfig) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await
        .with_context(|| "failed to connect to PostgreSQL")
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR
        .run(pool)
        .await
        .with_context(|| "failed to run database migrations")
}
