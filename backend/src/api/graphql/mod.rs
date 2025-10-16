//! GraphQL endpoint placeholder. Implemented with async-graphql once schema is finalized.

use axum::{routing::post, Router};

pub fn router() -> Router {
    Router::new().route("/graphql", post(handler))
}

async fn handler() -> &'static str {
    "GraphQL endpoint not yet implemented"
}
