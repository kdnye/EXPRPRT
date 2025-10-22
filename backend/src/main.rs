use std::net::SocketAddr;
use std::sync::Arc;

use axum::{serve, Extension};
use dotenvy::dotenv;
use expense_portal::{
    api,
    infrastructure::{config::Config, db, state::AppState, storage},
    jobs, telemetry,
};
use tokio::signal;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    telemetry::init();
    let config = Arc::new(Config::from_env()?);
    let pool = db::connect(&config.database).await?;
    db::run_migrations(&pool).await?;
    info!("database migrations completed successfully");
    let storage = storage::build_storage(&config.storage)?;
    let state = Arc::new(AppState::new(Arc::clone(&config), pool, storage)?);

    let router = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let addr: SocketAddr = config.bind_address().parse()?;
    info!(%addr, "starting expense portal api");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let _digest_handle = jobs::spawn_digest_worker(Arc::clone(&state));

    let server = serve(listener, router.into_make_service());

    tokio::select! {
        res = server => {
            if let Err(err) = res {
                warn!(error = ?err, "server exited with error");
            }
        }
        _ = shutdown_signal() => {
            info!("shutdown signal received");
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
