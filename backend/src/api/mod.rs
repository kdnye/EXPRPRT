use axum::{http::StatusCode, Json, Router};

use self::rest::router as rest_router;

pub mod graphql;
pub mod rest;

pub fn build_router() -> Router {
    Router::new().nest("/api", rest_router())
}

pub async fn not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "not_found"})),
    )
}
