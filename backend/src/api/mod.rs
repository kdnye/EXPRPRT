use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Request},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    Json, Router,
};
use tower_http::services::ServeDir;

use self::rest::router as rest_router;

pub mod graphql;
pub mod rest;

use crate::infrastructure::{
    auth::{AuthError, AuthenticatedUser},
    config::Config,
    storage,
};

pub fn build_router(config: Arc<Config>) -> Router {
    let router = Router::new().nest("/api", rest_router());

    if let Some(receipts_router) = receipts_router(config.as_ref()) {
        router.merge(receipts_router)
    } else {
        router
    }
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

async fn require_authenticated_user(request: Request, next: Next) -> Result<Response, AuthError> {
    let (mut parts, body) = request.into_parts();
    AuthenticatedUser::from_request_parts(&mut parts, &()).await?;
    let request = Request::from_parts(parts, body);
    Ok(next.run(request).await)
}
