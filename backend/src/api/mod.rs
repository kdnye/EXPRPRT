use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Request},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    Json, Router,
};
use tower_http::services::ServeDir;

use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing::warn;

use self::rest::router as rest_router;
pub mod rest;

use crate::infrastructure::{
    auth::{AuthError, AuthenticatedUser},
    config::Config,
    storage,
};

pub fn build_router(config: Arc<Config>) -> Router {
    let router = Router::new().nest("/api", rest_router());

    let router = if let Some(receipts_router) = receipts_router(config.as_ref()) {
        router.merge(receipts_router)
    } else {
        router
    };

    router.layer(build_cors_layer(config.as_ref()))
}

pub async fn not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "not_found"})),
    )
}

fn receipts_router(config: &Config) -> Option<Router> {
    if config.storage.provider != "local" {
        return None;
    }

    let root = storage::local_storage_root(config.storage.local_path.as_deref());
    let service = ServeDir::new(root).append_index_html_on_directories(false);

    Some(
        Router::new()
            .nest_service("/receipts", service)
            .layer(middleware::from_fn(require_authenticated_user)),
    )
}

fn build_cors_layer(config: &Config) -> CorsLayer {
    const DEFAULT_CORS_ORIGINS: &[&str] = &["http://localhost:5173", "http://127.0.0.1:5173"];

    let base = CorsLayer::new()
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    let configured_origins: Vec<&str> = if config.app.cors_origins.is_empty() {
        DEFAULT_CORS_ORIGINS.to_vec()
    } else {
        config.app.cors_origins.iter().map(String::as_str).collect()
    };

    let origins: Vec<HeaderValue> = configured_origins
        .into_iter()
        .filter_map(|origin| match origin.parse::<HeaderValue>() {
            Ok(value) => Some(value),
            Err(error) => {
                warn!(%origin, ?error, "skipping invalid CORS origin");
                None
            }
        })
        .collect();

    if origins.is_empty() {
        warn!("no valid CORS origins configured; credentialed requests will fail");
        base
    } else {
        base.allow_origin(AllowOrigin::list(origins))
    }
}

async fn require_authenticated_user(request: Request, next: Next) -> Result<Response, AuthError> {
    let (mut parts, body) = request.into_parts();
    AuthenticatedUser::from_request_parts(&mut parts, &()).await?;
    let request = Request::from_parts(parts, body);
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::build_cors_layer;
    use crate::infrastructure::config::{
        AppConfig, AuthConfig, Config, DatabaseConfig, NetSuiteConfig, ReceiptRules, StorageConfig,
    };

    fn base_config() -> Config {
        Config {
            app: AppConfig {
                cors_origins: vec!["http://example.com".into()],
                ..AppConfig::default()
            },
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
            storage: StorageConfig::default(),
            netsuite: NetSuiteConfig::default(),
            receipts: ReceiptRules::default(),
        }
    }

    #[test]
    fn cors_layer_with_credentials_does_not_panic_with_configured_origins() {
        let config = base_config();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build_cors_layer(&config)));

        assert!(result.is_ok(), "building the CORS layer should not panic");
    }
}
