use std::sync::Arc;

use axum::{extract::Extension, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{
    domain::models::{Employee, Role},
    infrastructure::{auth::issue_token, state::AppState},
    services::errors::ServiceError,
};

pub fn router() -> Router {
    Router::new().route("/login", post(login))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    hr_identifier: String,
    credential: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
    role: Role,
}

async fn login(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    if state.config.auth.developer_credential.is_empty()
        || payload.credential != state.config.auth.developer_credential
    {
        return Err(unauthorized());
    }

    let employee = sqlx::query_as::<_, Employee>(
        r#"
        SELECT id, hr_identifier, manager_id, department, role, created_at
        FROM employees
        WHERE hr_identifier = $1
        "#,
    )
    .bind(&payload.hr_identifier)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| to_response(ServiceError::Internal(err.to_string())))?;

    let Some(employee) = employee else {
        return Err(unauthorized());
    };

    let token = issue_token(&state, &employee).map_err(to_response)?;

    Ok(Json(LoginResponse {
        token,
        role: employee.role,
    }))
}

fn unauthorized() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "invalid_credentials" })),
    )
}

fn to_response(err: ServiceError) -> (StatusCode, Json<serde_json::Value>) {
    (
        err.status_code(),
        Json(serde_json::json!({ "error": err.to_string() })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_returns_expected_payload() {
        let (status, Json(body)) = unauthorized();

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body, serde_json::json!({ "error": "invalid_credentials" }));
    }
}
