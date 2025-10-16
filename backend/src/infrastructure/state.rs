use std::sync::Arc;

use crate::infrastructure::{auth::JwtKeys, config::Config, db::PgPool, storage::StorageBackend};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub storage: Arc<dyn StorageBackend>,
    pub jwt_keys: JwtKeys,
}

impl AppState {
    pub fn new(config: Arc<Config>, pool: PgPool, storage: Arc<dyn StorageBackend>) -> Self {
        let jwt_keys = JwtKeys::new(&config.auth.jwt_secret);
        Self {
            config,
            pool,
            storage,
            jwt_keys,
        }
    }
}
