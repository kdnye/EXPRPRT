use axum::{routing::get, Router};

use crate::api::rest::{
    approvals::router as approvals_router, expenses::router as expenses_router,
    finance::router as finance_router,
};

pub mod approvals;
pub mod expenses;
pub mod finance;
pub mod health;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health::healthcheck))
        .nest("/expenses", expenses_router())
        .nest("/approvals", approvals_router())
        .nest("/finance", finance_router())
}
