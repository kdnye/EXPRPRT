use axum::http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("conflict")]
    Conflict,
    #[error("internal error: {0}")]
    Internal(String),
}

impl ServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServiceError::NotFound => StatusCode::NOT_FOUND,
            ServiceError::Forbidden => StatusCode::FORBIDDEN,
            ServiceError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ServiceError::Conflict => StatusCode::CONFLICT,
            ServiceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
