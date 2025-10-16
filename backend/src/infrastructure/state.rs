use std::sync::Arc;

use crate::infrastructure::{config::Config, db::PgPool, storage::StorageBackend};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub storage: Arc<dyn StorageBackend>,
}

impl AppState {
    pub fn new(config: Arc<Config>, pool: PgPool, storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            config,
            pool,
            storage,
        }
    }
}
