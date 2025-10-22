use std::sync::Arc;

use anyhow::Result;
use sqlx::query_as;
use tokio::sync::OnceCell;
use tracing::warn;

use crate::{
    domain::models::Employee,
    infrastructure::{
        auth::{AuthenticatedUser, JwtKeys},
        config::Config,
        db::PgPool,
        storage::StorageBackend,
    },
};

pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub storage: Arc<dyn StorageBackend>,
    pub jwt_keys: JwtKeys,
    bypass_user: OnceCell<Option<AuthenticatedUser>>,
}

impl AppState {
    pub fn new(
        config: Arc<Config>,
        pool: PgPool,
        storage: Arc<dyn StorageBackend>,
    ) -> Result<Self> {
        if config.auth.jwt_secret.trim().is_empty() {
            anyhow::bail!(
                "JWT secret is blank. Set `config.auth.jwt_secret` or the `EXPENSES__AUTH__JWT_SECRET` environment variable."
            );
        }

        let jwt_keys = JwtKeys::new(&config.auth.jwt_secret);
        if config.auth.bypass_auth {
            if let Some(hr_identifier) = config
                .auth
                .bypass_hr_identifier
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                warn!(
                    hr_identifier,
                    "Authentication bypass enabled; requests will impersonate this employee"
                );
            } else {
                warn!(
                    "Authentication bypass enabled without a fallback employee; requests will be rejected"
                );
            }
        }
        Ok(Self {
            config,
            pool,
            storage,
            jwt_keys,
            bypass_user: OnceCell::new(),
        })
    }

    pub async fn resolve_bypass_user(&self) -> Result<Option<AuthenticatedUser>, sqlx::Error> {
        if !self.config.auth.bypass_auth {
            return Ok(None);
        }

        let Some(hr_identifier) = self
            .config
            .auth
            .bypass_hr_identifier
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };

        let normalized = hr_identifier.to_uppercase();
        let pool = self.pool.clone();
        let cached = self
            .bypass_user
            .get_or_try_init(|| {
                let pool = pool.clone();
                let normalized = normalized.clone();
                Box::pin(async move {
                    let employee = query_as::<_, Employee>(
                        r#"
                        SELECT id, hr_identifier, manager_id, department, role, created_at
                        FROM employees
                        WHERE UPPER(hr_identifier) = $1
                        "#,
                    )
                    .bind(&normalized)
                    .fetch_optional(&pool)
                    .await?;

                    match employee {
                        Some(employee) => {
                            Ok::<Option<AuthenticatedUser>, sqlx::Error>(Some(AuthenticatedUser {
                                employee_id: employee.id,
                                role: employee.role,
                            }))
                        }
                        None => {
                            warn!(
                                hr_identifier = %normalized,
                                "Authentication bypass employee not found"
                            );
                            Ok::<Option<AuthenticatedUser>, sqlx::Error>(None)
                        }
                    }
                })
            })
            .await?;

        Ok(cached.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{
        config::{
            AppConfig, AuthConfig, Config, DatabaseConfig, NetSuiteConfig, ReceiptRules,
            StorageConfig,
        },
        storage,
    };
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    fn build_pool() -> PgPool {
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://test:test@localhost:5432/test")
            .expect("failed to create lazy pool")
    }

    fn build_storage() -> Arc<dyn StorageBackend> {
        let mut storage_config = StorageConfig::default();
        storage_config.provider = "memory".to_string();
        storage::build_storage(&storage_config).expect("memory storage should build")
    }

    fn build_config(secret: &str) -> Arc<Config> {
        let mut storage_config = StorageConfig::default();
        storage_config.provider = "memory".to_string();

        Arc::new(Config {
            app: AppConfig::default(),
            database: DatabaseConfig {
                url: "postgres://test:test@localhost:5432/test".to_string(),
                max_connections: 1,
            },
            auth: AuthConfig {
                jwt_secret: secret.to_string(),
                ..AuthConfig::default()
            },
            storage: storage_config,
            netsuite: NetSuiteConfig::default(),
            receipts: ReceiptRules::default(),
        })
    }

    #[tokio::test]
    async fn new_rejects_blank_jwt_secret() {
        let config = build_config("   ");
        let pool = build_pool();
        let storage = build_storage();

        let result = AppState::new(config, pool, storage);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn new_accepts_populated_jwt_secret() {
        let config = build_config("integration-secret");
        let pool = build_pool();
        let storage = build_storage();

        let state = AppState::new(config, pool, storage);

        assert!(state.is_ok());
    }
}
