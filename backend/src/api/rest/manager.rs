use std::sync::Arc;

use axum::{extract::Extension, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;

use crate::{
    infrastructure::{auth::AuthenticatedUser, state::AppState},
    services::{
        errors::ServiceError,
        manager::{ManagerQueueEntry, ManagerService},
    },
};

pub fn router() -> Router {
    Router::new().route("/queue", get(queue))
}

async fn queue(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
) -> Result<Json<ManagerQueueResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = ManagerService::new(state);
    let queue = service.fetch_queue(&user).await.map_err(to_response)?;

    Ok(Json(ManagerQueueResponse { queue }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagerQueueResponse {
    queue: Vec<ManagerQueueEntry>,
}

fn to_response(err: ServiceError) -> (StatusCode, Json<serde_json::Value>) {
    (
        err.status_code(),
        Json(serde_json::json!({ "error": err.to_string() })),
    )
}
