use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    infrastructure::{auth::AuthenticatedUser, state::AppState},
    services::errors::ServiceError,
    services::expenses::{CreateReportRequest, ExpenseService},
};

pub fn router() -> Router {
    Router::new()
        .route("/reports", post(create_report))
        .route("/reports/:id/submit", post(submit_report))
        .route("/reports/:id/policy", get(evaluate_report))
}

async fn create_report(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateReportRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = ExpenseService::new(state);
    let report = service
        .create_report(&user, payload)
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "report": report })))
}

async fn submit_report(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = ExpenseService::new(state);
    let report = service
        .submit_report(&user, id)
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "report": report })))
}

async fn evaluate_report(
    Extension(state): Extension<Arc<AppState>>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = ExpenseService::new(state);
    let result = service.evaluate_report(id).await.map_err(to_response)?;
    Ok(Json(serde_json::json!({ "evaluation": result })))
}

fn to_response(err: ServiceError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        err.status_code(),
        Json(serde_json::json!({ "error": err.to_string() })),
    )
}
