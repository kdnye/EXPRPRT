use std::sync::Arc;

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
    pub fn new(config: Arc<Config>, pool: PgPool, storage: Arc<dyn StorageBackend>) -> Self {
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
        Self {
            config,
            pool,
            storage,
            jwt_keys,
            bypass_user: OnceCell::new(),
        }
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
