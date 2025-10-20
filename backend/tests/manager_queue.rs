use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Extension,
};
use chrono::{Duration, NaiveDate, Utc};
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
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn manager_queue_requires_manager_role() -> Result<()> {
    let Some(pool) = maybe_connect_pool().await? else {
        return Ok(());
    };

    sqlx::migrate!("./migrations").run(&pool).await?;

    run_requires_manager(pool).await
}

#[tokio::test]
async fn manager_queue_returns_pending_reports() -> Result<()> {
    let Some(pool) = maybe_connect_pool().await? else {
        return Ok(());
    };

    sqlx::migrate!("./migrations").run(&pool).await?;

    run_happy_path(pool).await
}

async fn maybe_connect_pool() -> Result<Option<PgPool>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("EXPENSES__DATABASE__URL"))
        .unwrap_or_else(|_| "postgres://expenses:expenses@localhost:5432/expenses".to_string());

    match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => Ok(Some(pool)),
        Err(err) => {
            eprintln!("Skipping integration test: unable to connect to database: {err}");
            Ok(None)
        }
    }
}

async fn run_requires_manager(pool: PgPool) -> Result<()> {
    let (config, state) = build_state(pool.clone()).await?;
    let app = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let employee_id = Uuid::new_v4();
    let hr_identifier = format!("EMP-{}", employee_id.simple());

    sqlx::query(
        "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(employee_id)
    .bind(&hr_identifier)
    .bind::<Option<Uuid>>(None)
    .bind::<Option<String>>(None)
    .bind(Role::Employee)
    .bind(Utc::now())
    .execute(&pool)
    .await?;

    let employee = fetch_employee(&pool, employee_id).await?;
    let token = issue_token(&state, &employee)?;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/manager/queue")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("service error");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    sqlx::query("DELETE FROM employees WHERE id = $1")
        .bind(employee_id)
        .execute(&pool)
        .await?;

    Ok(())
}

async fn run_happy_path(pool: PgPool) -> Result<()> {
    let (config, state) = build_state(pool.clone()).await?;
    let app = api::build_router(Arc::clone(&config)).layer(Extension(Arc::clone(&state)));

    let manager_id = Uuid::new_v4();
    let employee_id = Uuid::new_v4();
    let report_id = Uuid::new_v4();
    let flagged_item_id = Uuid::new_v4();
    let regular_item_id = Uuid::new_v4();

    let manager_hr = format!("MGMT-{}", manager_id.simple());
    let employee_hr = format!("EMP-{}", employee_id.simple());

    sqlx::query(
        "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(manager_id)
    .bind(&manager_hr)
    .bind::<Option<Uuid>>(None)
    .bind::<Option<String>>(Some("Operations".to_string()))
    .bind(Role::Manager)
    .bind(Utc::now())
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(employee_id)
    .bind(&employee_hr)
    .bind::<Option<Uuid>>(Some(manager_id))
    .bind::<Option<String>>(Some("Logistics".to_string()))
    .bind(Role::Employee)
    .bind(Utc::now())
    .execute(&pool)
    .await?;

    let submitted_at = Utc::now() - Duration::days(2);
    let period_start = NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date");
    let period_end = NaiveDate::from_ymd_opt(2024, 5, 31).expect("valid date");

    sqlx::query(
        "INSERT INTO expense_reports
             (id, employee_id, reporting_period_start, reporting_period_end, status,
              total_amount_cents, total_reimbursable_cents, currency, version, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"
    )
    .bind(report_id)
    .bind(employee_id)
    .bind(period_start)
    .bind(period_end)
    .bind("submitted")
    .bind(85_000_i64)
    .bind(65_000_i64)
    .bind("USD")
    .bind(1_i32)
    .bind(submitted_at)
    .bind(submitted_at)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO expense_items
             (id, report_id, expense_date, category, description, attendees, location,
              amount_cents, reimbursable, payment_method, is_policy_exception)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(regular_item_id)
    .bind(report_id)
    .bind(period_start)
    .bind("meal")
    .bind(Some("Team lunch".to_string()))
    .bind::<Option<String>>(None)
    .bind::<Option<String>>(Some("Denver".to_string()))
    .bind(18_500_i64)
    .bind(true)
    .bind::<Option<String>>(Some("corporate_card".to_string()))
    .bind(false)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO expense_items
             (id, report_id, expense_date, category, description, attendees, location,
              amount_cents, reimbursable, payment_method, is_policy_exception)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(flagged_item_id)
    .bind(report_id)
    .bind(period_start.succ_opt().expect("date"))
    .bind("lodging")
    .bind(Some("Hotel over cap".to_string()))
    .bind::<Option<String>>(None)
    .bind::<Option<String>>(Some("Denver".to_string()))
    .bind(46_500_i64)
    .bind(true)
    .bind::<Option<String>>(Some("personal_card".to_string()))
    .bind(true)
    .execute(&pool)
    .await?;

    let manager = fetch_employee(&pool, manager_id).await?;
    let token = issue_token(&state, &manager)?;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/manager/queue")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("service error");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let payload: Value = serde_json::from_slice(&body)?;
    let queue = payload
        .get("queue")
        .and_then(Value::as_array)
        .expect("queue array");

    assert_eq!(queue.len(), 1);
    let entry = &queue[0];
    let report = entry.get("report").expect("report section");
    assert_eq!(
        report.get("id").and_then(Value::as_str),
        Some(report_id.to_string()).as_deref()
    );
    assert_eq!(
        report.get("employeeHrIdentifier").and_then(Value::as_str),
        Some(employee_hr.as_str())
    );
    assert_eq!(
        report.get("reportingPeriodStart").and_then(Value::as_str),
        Some(period_start.to_string()).as_deref()
    );
    assert_eq!(
        report.get("reportingPeriodEnd").and_then(Value::as_str),
        Some(period_end.to_string()).as_deref()
    );
    assert_eq!(
        report.get("totalAmountCents").and_then(Value::as_i64),
        Some(85_000_i64)
    );
    assert_eq!(
        report.get("totalReimbursableCents").and_then(Value::as_i64),
        Some(65_000_i64)
    );
    assert_eq!(report.get("currency").and_then(Value::as_str), Some("USD"));

    let submitted_value = report
        .get("submittedAt")
        .and_then(Value::as_str)
        .expect("submitted at iso8601");
    let parsed_submitted = chrono::DateTime::parse_from_rfc3339(submitted_value)
        .expect("valid RFC3339 timestamp")
        .with_timezone(&Utc);
    assert_eq!(parsed_submitted, submitted_at);

    let line_items = entry
        .get("lineItems")
        .and_then(Value::as_array)
        .expect("line items");
    assert_eq!(line_items.len(), 2);
    let first_item = &line_items[0];
    assert_eq!(
        first_item.get("id").and_then(Value::as_str),
        Some(regular_item_id.to_string().as_str())
    );
    assert_eq!(
        first_item.get("paymentMethod").and_then(Value::as_str),
        Some("corporate_card")
    );
    assert_eq!(
        first_item.get("isPolicyException").and_then(Value::as_bool),
        Some(false)
    );

    let policy_flags = entry
        .get("policyFlags")
        .and_then(Value::as_array)
        .expect("policy flags");
    assert_eq!(policy_flags.len(), 1);
    let flag = &policy_flags[0];
    assert_eq!(
        flag.get("itemId").and_then(Value::as_str),
        Some(flagged_item_id.to_string().as_str())
    );
    assert_eq!(
        flag.get("category").and_then(Value::as_str),
        Some("lodging")
    );

    sqlx::query("DELETE FROM expense_items WHERE report_id = $1")
        .bind(report_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM expense_reports WHERE id = $1")
        .bind(report_id)
        .execute(&pool)
        .await?;
    let employee_ids = vec![employee_id, manager_id];
    sqlx::query("DELETE FROM employees WHERE id = ANY($1)")
        .bind(&employee_ids)
        .execute(&pool)
        .await?;

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
        },
        storage: storage_config,
        netsuite: NetSuiteConfig::default(),
        receipts: ReceiptRules::default(),
    });

    let storage = storage::build_storage(&config.storage)?;
    let state = Arc::new(AppState::new(Arc::clone(&config), pool, storage));

    Ok((config, state))
}

async fn fetch_employee(pool: &PgPool, id: Uuid) -> Result<Employee> {
    let employee = sqlx::query_as::<_, Employee>(
        "SELECT id, hr_identifier, manager_id, department, role, created_at FROM employees WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(employee)
}
