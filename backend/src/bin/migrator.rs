use dotenvy::dotenv;
use expense_portal::{
    infrastructure::{config::Config, db},
    telemetry,
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    telemetry::init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database).await?;
    db::run_migrations(&pool).await?;

    info!("database migrations completed");

    Ok(())
}
