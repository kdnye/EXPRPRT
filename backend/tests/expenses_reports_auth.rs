use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Extension,
};
use chrono::Utc;
use expense_portal::{
    api,
    domain::models::Role,
    infrastructure::{
        config::{
            AppConfig, AuthConfig, Config, DatabaseConfig, NetSuiteConfig, ReceiptRules,
            StorageConfig,
        },
        state::AppState,
        storage,
    },
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "test_harness.rs"]
mod test_harness;

use test_harness::run_test;

#[tokio::test]
async fn expenses_reports_requires_valid_token() -> Result<()> {
    run_test(run_scenario).await
}

async fn run_scenario(pool: PgPool) -> Result<()> {
    let mut storage_config = StorageConfig::default();
    storage_config.provider = "memory".to_string();

    let config = Arc::new(Config {
        app: AppConfig::default(),
        database: DatabaseConfig {
            url: "postgres://integration".to_string(),
            max_connections: 5,
        },
        auth: AuthConfig {
            jwt_secret: "integration-secret".to_string(),
            jwt_ttl_seconds: 3_600,
            developer_credential: "dev-pass".to_string(),
            bypass_auth: false,
            bypass_hr_identifier: None,
        },
        storage: storage_config,
        netsuite: NetSuiteConfig::default(),
        receipts: ReceiptRules::default(),
    });

    let storage = storage::build_storage(&config.storage)?;
    let state = Arc::new(AppState::new(Arc::clone(&config), pool.clone(), storage));

    let hr_identifier = format!("DEV{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(&hr_identifier)
    .bind::<Option<Uuid>>(None)
    .bind::<Option<String>>(None)
    .bind(Role::Employee)
    .bind(Utc::now())
    .execute(&pool)
    .await?;

    let app = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let unauthenticated_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/expenses/reports")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "reporting_period_start": "2024-01-01",
                        "reporting_period_end": "2024-01-31",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .expect("failed to build unauthenticated request"),
        )
        .await
        .expect("service error");

    assert_eq!(unauthenticated_response.status(), StatusCode::UNAUTHORIZED);

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "hr_identifier": hr_identifier,
                        "credential": "dev-pass"
                    })
                    .to_string(),
                ))
                .expect("failed to build login request"),
        )
        .await
        .expect("service error");

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = to_bytes(login_response.into_body(), 1024 * 1024).await?;
    let token: String = serde_json::from_slice::<Value>(&login_body)?
        .get("token")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();

    let authorized_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/expenses/reports")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    serde_json::json!({
                        "reporting_period_start": "2024-01-01",
                        "reporting_period_end": "2024-01-31",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .expect("failed to build authorized request"),
        )
        .await
        .expect("service error");

    assert_eq!(authorized_response.status(), StatusCode::OK);

    sqlx::query("DELETE FROM employees WHERE hr_identifier = $1")
        .bind(hr_identifier)
        .execute(&pool)
        .await?;

    Ok(())
}
