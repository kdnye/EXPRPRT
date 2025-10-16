use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

use crate::infrastructure::state::AppState;

pub fn spawn_digest_worker(_state: Arc<AppState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            info!("digest worker stub running");
            tokio::time::sleep(std::time::Duration::from_secs(60 * 60 * 24)).await;
        }
    })
}
