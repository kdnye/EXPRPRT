use std::sync::Arc;

use axum::{extract::Extension, routing::post, Json, Router};

use crate::{
    infrastructure::auth::AuthenticatedUser,
    infrastructure::state::AppState,
    services::{
        errors::ServiceError,
        finance::{FinalizeRequest, FinanceService},
    },
};

pub fn router() -> Router {
    Router::new().route("/finalize", post(finalize))
}

async fn finalize(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(payload): Json<FinalizeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = FinanceService::new(state);
    let batch = service
        .finalize_reports(&user, payload)
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "batch": batch })))
}

fn to_response(err: ServiceError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        err.status_code(),
        Json(serde_json::json!({ "error": err.to_string() })),
    )
}
