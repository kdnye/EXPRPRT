use std::sync::Arc;

use axum::{extract::Extension, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

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
    let Some(hr_identifier) = normalize_hr_identifier(&payload.hr_identifier) else {
        return Err(unauthorized());
    };

    let credential = payload.credential.trim();
    if credential.is_empty() {
        return Err(unauthorized());
    }

    let configured_credential = state.config.auth.developer_credential.trim();
    if configured_credential.is_empty()
        || !bool::from(
            credential
                .as_bytes()
                .ct_eq(configured_credential.as_bytes()),
        )
    {
        return Err(unauthorized());
    }

    let employee = sqlx::query_as::<_, Employee>(
        r#"
        SELECT id, hr_identifier, manager_id, department, role, created_at
        FROM employees
        WHERE UPPER(hr_identifier) = $1
        "#,
    )
    .bind(&hr_identifier)
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

fn normalize_hr_identifier(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_uppercase())
}

fn unauthorized() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "invalid_credentials" })),
    )
}

fn to_response(err: ServiceError) -> (StatusCode, Json<serde_json::Value>) {
    let status = err.status_code();
    let body = match err {
        ServiceError::Internal(e) => {
            tracing::error!("Internal error: {}", e);
            serde_json::json!({ "error": "internal_server_error" })
        }
        _ => serde_json::json!({ "error": err.to_string() }),
    };
    (status, Json(body))
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

    #[test]
    fn normalize_hr_identifier_trims_and_uppercases() {
        let input = "  mgmt1001\t";
        let normalized = normalize_hr_identifier(input);
        assert_eq!(normalized.as_deref(), Some("MGMT1001"));
    }

    #[test]
    fn normalize_hr_identifier_rejects_blank_input() {
        assert_eq!(normalize_hr_identifier("   "), None);
    }
}
