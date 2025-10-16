use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    routing::post,
    Json, Router,
};
use uuid::Uuid;

use crate::{
    infrastructure::auth::AuthenticatedUser,
    infrastructure::state::AppState,
    services::{
        approvals::{ApprovalService, DecisionRequest},
        errors::ServiceError,
    },
};

pub fn router() -> Router {
    Router::new().route("/:id", post(decide))
}

async fn decide(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<DecisionRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = ApprovalService::new(state);
    let approval = service
        .record_decision(&user, id, payload)
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "approval": approval })))
}

fn to_response(err: ServiceError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        err.status_code(),
        Json(serde_json::json!({ "error": err.to_string() })),
    )
}
