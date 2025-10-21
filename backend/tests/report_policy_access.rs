use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Extension,
};
use chrono::{NaiveDate, Utc};
use expense_portal::{
    api,
    domain::models::{Employee, Role},
    infrastructure::{
        auth::issue_token,
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
async fn report_policy_allows_owner() -> Result<()> {
    run_test(run_owner_access).await
}

#[tokio::test]
async fn report_policy_blocks_unrelated_employee() -> Result<()> {
    run_test(run_cross_employee_forbidden).await
}

#[tokio::test]
async fn report_policy_allows_manager() -> Result<()> {
    run_test(|pool| run_reviewer_access(pool, Role::Manager)).await
}

#[tokio::test]
async fn report_policy_allows_finance() -> Result<()> {
    run_test(|pool| run_reviewer_access(pool, Role::Finance)).await
}

async fn run_owner_access(pool: PgPool) -> Result<()> {
    let (config, state) = build_state(pool.clone()).await?;
    let app = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let owner = create_employee(&pool, Role::Employee).await?;
    let report_id = create_report_with_item(&pool, owner.id).await?;
    let token = issue_token(&state, &owner)?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/expenses/reports/{report_id}/policy"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("failed to build owner request"),
        )
        .await
        .expect("service error");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let payload: Value = serde_json::from_slice(&body)?;
    assert!(payload.get("evaluation").is_some(), "evaluation missing");

    cleanup(&pool, report_id, &[owner.id]).await?;

    Ok(())
}

async fn run_cross_employee_forbidden(pool: PgPool) -> Result<()> {
    let (config, state) = build_state(pool.clone()).await?;
    let app = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let owner = create_employee(&pool, Role::Employee).await?;
    let other_employee = create_employee(&pool, Role::Employee).await?;
    let report_id = create_report_with_item(&pool, owner.id).await?;
    let token = issue_token(&state, &other_employee)?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/expenses/reports/{report_id}/policy"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("failed to build unrelated employee request"),
        )
        .await
        .expect("service error");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    cleanup(&pool, report_id, &[owner.id, other_employee.id]).await?;

    Ok(())
}

async fn run_reviewer_access(pool: PgPool, role: Role) -> Result<()> {
    let (config, state) = build_state(pool.clone()).await?;
    let app = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let owner = create_employee(&pool, Role::Employee).await?;
    let reviewer = create_employee(&pool, role).await?;
    let report_id = create_report_with_item(&pool, owner.id).await?;
    let token = issue_token(&state, &reviewer)?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/expenses/reports/{report_id}/policy"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("failed to build reviewer request"),
        )
        .await
        .expect("service error");

    assert_eq!(response.status(), StatusCode::OK);

    cleanup(&pool, report_id, &[owner.id, reviewer.id]).await?;

    Ok(())
}

async fn build_state(pool: PgPool) -> Result<(Arc<Config>, Arc<AppState>)> {
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
    let state = Arc::new(AppState::new(Arc::clone(&config), pool, storage));

    Ok((config, state))
}

async fn create_employee(pool: &PgPool, role: Role) -> Result<Employee> {
    let id = Uuid::new_v4();
    let prefix = match role {
        Role::Employee => "EMP",
        Role::Manager => "MGMT",
        Role::Finance => "FIN",
        Role::Admin => "ADMIN",
    };
    let hr_identifier = format!("{prefix}-{}", id.simple());

    sqlx::query(
        "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(&hr_identifier)
    .bind::<Option<Uuid>>(None)
    .bind::<Option<String>>(None)
    .bind(role)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    let employee = sqlx::query_as::<_, Employee>(
        "SELECT id, hr_identifier, manager_id, department, role, created_at FROM employees WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(employee)
}

async fn create_report_with_item(pool: &PgPool, employee_id: Uuid) -> Result<Uuid> {
    let report_id = Uuid::new_v4();
    let created_at = Utc::now();
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid start date");
    let end = NaiveDate::from_ymd_opt(2024, 1, 31).expect("valid end date");

    sqlx::query(
        "INSERT INTO expense_reports
             (id, employee_id, reporting_period_start, reporting_period_end, status,
              total_amount_cents, total_reimbursable_cents, currency, version, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(report_id)
    .bind(employee_id)
    .bind(start)
    .bind(end)
    .bind("draft")
    .bind(12_500_i64)
    .bind(12_500_i64)
    .bind("USD")
    .bind(1_i32)
    .bind(created_at)
    .bind(created_at)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO expense_items
             (id, report_id, expense_date, category, description, attendees, location,
              amount_cents, reimbursable, payment_method, is_policy_exception)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(Uuid::new_v4())
    .bind(report_id)
    .bind(start)
    .bind("meal")
    .bind(Some("Client lunch".to_string()))
    .bind::<Option<String>>(None)
    .bind::<Option<String>>(Some("Denver".to_string()))
    .bind(12_500_i64)
    .bind(true)
    .bind::<Option<String>>(Some("personal_card".to_string()))
    .bind(false)
    .execute(pool)
    .await?;

    Ok(report_id)
}

async fn cleanup(pool: &PgPool, report_id: Uuid, employee_ids: &[Uuid]) -> Result<()> {
    sqlx::query("DELETE FROM expense_items WHERE report_id = $1")
        .bind(report_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM expense_reports WHERE id = $1")
        .bind(report_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM employees WHERE id = ANY($1)")
        .bind(employee_ids)
        .execute(pool)
        .await?;

    Ok(())
}
