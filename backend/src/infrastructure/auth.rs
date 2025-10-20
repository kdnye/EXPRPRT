use std::sync::Arc;

use axum::{
    async_trait, extract::FromRequestParts, http::request::Parts, response::IntoResponse, Json,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

use crate::{
    domain::models::{Employee, Role},
    infrastructure::state::AppState,
    services::errors::ServiceError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: uuid::Uuid,
    pub role: Role,
    pub exp: usize,
}

#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}

pub fn issue_token(state: &AppState, employee: &Employee) -> Result<String, ServiceError> {
    let expiration = chrono::Utc::now()
        + chrono::Duration::from_std(state.config.jwt_ttl())
            .map_err(|_| ServiceError::Internal("failed to calculate expiration".into()))?;
    let claims = Claims {
        sub: employee.id,
        role: employee.role.clone(),
        exp: expiration.timestamp() as usize,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &state.jwt_keys.encoding,
    )
    .map_err(|err| ServiceError::Internal(err.to_string()))
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("missing authorization header")]
    Missing,
    #[error("invalid authorization token")]
    Invalid,
    #[error("missing application state")]
    MissingState,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status = axum::http::StatusCode::UNAUTHORIZED;
        let message = match self {
            AuthError::Missing => "missing authorization header",
            AuthError::Invalid => "invalid authorization token",
            AuthError::MissingState => "application state unavailable",
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub employee_id: uuid::Uuid,
    pub role: Role,
}

#[async_trait]
impl FromRequestParts<()> for AuthenticatedUser {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &()) -> Result<Self, Self::Rejection> {
        let Some(state) = parts.extensions.get::<Arc<AppState>>() else {
            return Err(AuthError::MissingState);
        };

        match state.resolve_bypass_user().await {
            Ok(Some(user)) => return Ok(user),
            Ok(None) => {}
            Err(err) => {
                warn!(error = ?err, "failed to resolve bypass user");
            }
        }

        let Some(header_value) = parts.headers.get(axum::http::header::AUTHORIZATION) else {
            return Err(AuthError::Missing);
        };
        let header_str = header_value.to_str().map_err(|_| AuthError::Invalid)?;
        let token = header_str
            .strip_prefix("Bearer ")
            .ok_or(AuthError::Invalid)?;
        let validation = Validation::new(Algorithm::HS256);
        match decode::<Claims>(token, &state.jwt_keys.decoding, &validation) {
            Ok(data) => Ok(AuthenticatedUser {
                employee_id: data.claims.sub,
                role: data.claims.role,
            }),
            Err(err) => {
                warn!(error = ?err, "failed to decode jwt");
                Err(AuthError::Invalid)
            }
        }
    }
}
