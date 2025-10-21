//! Coordinates expense report submission and policy evaluation workflows.
//!
//! This service powers the REST handlers mounted under `/reports`,
//! `/reports/:id/submit`, and `/reports/:id/policy` in
//! `backend/src/api/rest/expenses.rs`, stitching together persistence and
//! domain policy checks so UI flows can surface actionable results.

use std::{collections::HashSet, sync::Arc};

use chrono::Utc;
use serde::Deserialize;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

use crate::{
    domain::{
        models::{ExpenseCategory, ExpenseItem, ExpenseReport, PolicyCap, ReportStatus, Role},
        policy::{evaluate_item, PolicyEvaluation},
    },
    infrastructure::state::AppState,
};

use super::errors::ServiceError;

/// Request payload accepted by `POST /reports` for starting a draft report.
///
/// The reporting period window is later enforced against the approval flow
/// described in `POLICY.md` §"Approvals and Reimbursement Process" so finance
/// reviewers can reconcile period-close timelines.
#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub reporting_period_start: chrono::NaiveDate,
    pub reporting_period_end: chrono::NaiveDate,
    pub currency: String,
    #[serde(default)]
    pub items: Vec<CreateExpenseItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateExpenseItem {
    pub expense_date: chrono::NaiveDate,
    pub category: ExpenseCategory,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attendees: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    pub amount_cents: i64,
    pub reimbursable: bool,
    #[serde(default)]
    pub payment_method: Option<String>,
    #[serde(default)]
    pub receipts: Vec<CreateReceiptReference>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateReceiptReference {
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
}

/// Business façade around persistence and policy evaluation required to move
/// an expense report from draft through submission.
pub struct ExpenseService {
    pub state: Arc<AppState>,
}

impl ExpenseService {
    /// Builds a new expense service with the shared application state holding
    /// database pools and policy caches.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Creates a draft expense report for the authenticated employee.
    ///
    /// * `actor` — employee identity from the session, used to scope the new
    ///   record.
    /// * `payload` — reporting window and currency details supplied by the UI.
    ///
    /// Side effects:
    /// * Persists a `ReportStatus::Draft` row and initializes totals to zero.
    /// * Establishes the temporal boundaries referenced by
    ///   `POLICY.md` §"Approvals and Reimbursement Process" and subsequent
    ///   manager reviews.
    pub async fn create_report(
        &self,
        actor: &crate::infrastructure::auth::AuthenticatedUser,
        payload: CreateReportRequest,
    ) -> Result<ExpenseReport, ServiceError> {
        let mut tx = self
            .state
            .pool
            .begin()
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let status = ReportStatus::Draft;

        let CreateReportRequest {
            reporting_period_start,
            reporting_period_end,
            currency,
            items,
        } = payload;

        let (total_amount_cents, total_reimbursable_cents) = calculate_totals(&items);

        let record = sqlx::query(
            "INSERT INTO expense_reports (id, employee_id, reporting_period_start, reporting_period_end, status, total_amount_cents, total_reimbursable_cents, currency, version, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             RETURNING *",
        )
        .bind(id)
        .bind(actor.employee_id)
        .bind(reporting_period_start)
        .bind(reporting_period_end)
        .bind(status)
        .bind(total_amount_cents)
        .bind(total_reimbursable_cents)
        .bind(&currency)
        .bind(1_i32)
        .bind(now)
        .bind(now)
        .map(|row: PgRow| map_report(row))
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        for item in items {
            let item_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO expense_items (id, report_id, expense_date, category, gl_account_id, description, attendees, location, amount_cents, reimbursable, payment_method, is_policy_exception)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            )
            .bind(item_id)
            .bind(id)
            .bind(item.expense_date)
            .bind(item.category)
            .bind::<Option<Uuid>>(None)
            .bind(item.description)
            .bind(item.attendees)
            .bind(item.location)
            .bind(item.amount_cents)
            .bind(item.reimbursable)
            .bind(item.payment_method)
            .bind(false)
            .execute(&mut *tx)
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;

            for receipt in item.receipts {
                sqlx::query(
                    "INSERT INTO receipts (id, expense_item_id, file_key, file_name, mime_type, size_bytes, uploaded_by)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(Uuid::new_v4())
                .bind(item_id)
                .bind(receipt.file_key)
                .bind(receipt.file_name)
                .bind(receipt.mime_type)
                .bind(receipt.size_bytes)
                .bind(actor.employee_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| ServiceError::Internal(err.to_string()))?;
            }
        }

        tx.commit()
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;

        Ok(record)
    }

    /// Submits a draft report for approval by promoting it to
    /// `ReportStatus::Submitted`.
    ///
    /// * `actor` — employee requesting submission; must own the report.
    /// * `report_id` — identifier for the draft being submitted.
    ///
    /// The transition unlocks the manager approval gate noted in
    /// `POLICY.md` §"Approvals and Reimbursement Process". If the actor no
    /// longer owns the report or the status has changed, conflicts are surfaced
    /// back to the REST caller for UI resolution.
    pub async fn submit_report(
        &self,
        actor: &crate::infrastructure::auth::AuthenticatedUser,
        report_id: Uuid,
    ) -> Result<ExpenseReport, ServiceError> {
        let record = sqlx::query(
            "UPDATE expense_reports SET status=$1, version=version+1, updated_at=$2 WHERE id=$3 AND employee_id=$4 AND status='draft' RETURNING *",
        )
        .bind(ReportStatus::Submitted)
        .bind(Utc::now())
        .bind(report_id)
        .bind(actor.employee_id)
        .map(|row: PgRow| map_report(row))
        .fetch_optional(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        if let Some(record) = record {
            return Ok(record);
        }

        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM expense_reports WHERE id = $1 AND employee_id = $2",
        )
        .bind(report_id)
        .bind(actor.employee_id)
        .fetch_one(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        if exists == 0 {
            Err(ServiceError::NotFound)
        } else {
            Err(ServiceError::Conflict)
        }
    }

    /// Evaluates all items in the specified report against the policy engine.
    ///
    /// * `report_id` — identifies which report to aggregate.
    ///
    /// Side effects:
    /// * Reads the associated items and applicable `PolicyCap` records.
    /// * Delegates per-item checks to `domain::policy::evaluate_item`, which
    ///   encodes rules such as meal per-diem limits documented in
    ///   `POLICY.md` §"Meals" and mileage thresholds in §"Other Transportation".
    ///
    /// Returns a merged `PolicyEvaluation` describing violations and warnings
    /// that upstream REST handlers serialize for the UI.
    pub async fn evaluate_report(
        &self,
        actor: &crate::infrastructure::auth::AuthenticatedUser,
        report_id: Uuid,
    ) -> Result<PolicyEvaluation, ServiceError> {
        let owner_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT employee_id FROM expense_reports WHERE id = $1",
        )
        .bind(report_id)
        .fetch_optional(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        let Some(owner_id) = owner_id else {
            return Err(ServiceError::NotFound);
        };

        let is_reviewer = matches!(actor.role, Role::Manager | Role::Finance | Role::Admin);
        if actor.employee_id != owner_id && !is_reviewer {
            return Err(ServiceError::Forbidden);
        }

        let item_rows = sqlx::query(
            r#"
            SELECT id, report_id, expense_date, category, gl_account_id, description,
                   attendees, location, amount_cents, reimbursable, payment_method, is_policy_exception
            FROM expense_items
            WHERE report_id = $1
            "#,
        )
        .bind(report_id)
        .fetch_all(&self.state.pool)
        .await
        .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(item_rows.len());
        for row in item_rows {
            items.push(map_expense_item(row)?);
        }

        if items.is_empty() {
            return Ok(PolicyEvaluation::ok());
        }

        let mut category_keys: HashSet<ExpenseCategory> = HashSet::new();
        for item in &items {
            category_keys.insert(item.category);
        }
        let categories: Vec<ExpenseCategory> = category_keys.into_iter().collect();

        let cap_rows = if categories.is_empty() {
            Vec::new()
        } else {
            sqlx::query(
                r#"
                SELECT id, policy_key, category, limit_type, amount_cents, notes, active_from, active_to
                FROM policy_caps
                WHERE category = ANY($1)
                "#,
            )
            .bind(categories)
            .fetch_all(&self.state.pool)
            .await
            .map_err(map_sqlx_error)?
        };

        let mut caps = Vec::with_capacity(cap_rows.len());
        for row in cap_rows {
            caps.push(map_policy_cap(row)?);
        }

        Ok(aggregate_policy_evaluation(&items, &caps))
    }
}

fn calculate_totals(items: &[CreateExpenseItem]) -> (i64, i64) {
    let mut total_amount = 0_i64;
    let mut total_reimbursable = 0_i64;

    for item in items {
        total_amount += item.amount_cents;
        if item.reimbursable {
            total_reimbursable += item.amount_cents;
        }
    }

    (total_amount, total_reimbursable)
}

fn map_report(row: PgRow) -> ExpenseReport {
    ExpenseReport {
        id: row.get("id"),
        employee_id: row.get("employee_id"),
        reporting_period_start: row.get("reporting_period_start"),
        reporting_period_end: row.get("reporting_period_end"),
        status: row.get("status"),
        total_amount_cents: row.get("total_amount_cents"),
        total_reimbursable_cents: row.get("total_reimbursable_cents"),
        currency: row.get("currency"),
        version: row.get("version"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_expense_item(row: PgRow) -> Result<ExpenseItem, ServiceError> {
    let category = row
        .try_get::<ExpenseCategory, _>("category")
        .map_err(map_sqlx_error)?;
    Ok(ExpenseItem {
        id: row.try_get("id").map_err(map_sqlx_error)?,
        report_id: row.try_get("report_id").map_err(map_sqlx_error)?,
        expense_date: row.try_get("expense_date").map_err(map_sqlx_error)?,
        category,
        gl_account_id: row
            .try_get::<Option<Uuid>, _>("gl_account_id")
            .map_err(map_sqlx_error)?,
        description: row
            .try_get::<Option<String>, _>("description")
            .map_err(map_sqlx_error)?,
        attendees: row
            .try_get::<Option<String>, _>("attendees")
            .map_err(map_sqlx_error)?,
        location: row
            .try_get::<Option<String>, _>("location")
            .map_err(map_sqlx_error)?,
        amount_cents: row
            .try_get::<i64, _>("amount_cents")
            .map_err(map_sqlx_error)?,
        reimbursable: row
            .try_get::<bool, _>("reimbursable")
            .map_err(map_sqlx_error)?,
        payment_method: row
            .try_get::<Option<String>, _>("payment_method")
            .map_err(map_sqlx_error)?,
        is_policy_exception: row
            .try_get::<bool, _>("is_policy_exception")
            .map_err(map_sqlx_error)?,
    })
}

fn map_policy_cap(row: PgRow) -> Result<PolicyCap, ServiceError> {
    let category = row
        .try_get::<ExpenseCategory, _>("category")
        .map_err(map_sqlx_error)?;
    Ok(PolicyCap {
        id: row.try_get("id").map_err(map_sqlx_error)?,
        policy_key: row.try_get("policy_key").map_err(map_sqlx_error)?,
        category,
        limit_type: row
            .try_get::<String, _>("limit_type")
            .map_err(map_sqlx_error)?,
        amount_cents: row
            .try_get::<i64, _>("amount_cents")
            .map_err(map_sqlx_error)?,
        notes: row
            .try_get::<Option<String>, _>("notes")
            .map_err(map_sqlx_error)?,
        active_from: row
            .try_get::<chrono::NaiveDate, _>("active_from")
            .map_err(map_sqlx_error)?,
        active_to: row
            .try_get::<Option<chrono::NaiveDate>, _>("active_to")
            .map_err(map_sqlx_error)?,
    })
}

fn aggregate_policy_evaluation(items: &[ExpenseItem], caps: &[PolicyCap]) -> PolicyEvaluation {
    let mut evaluation = PolicyEvaluation::ok();

    for item in items {
        let item_evaluation = evaluate_item(item, caps);
        evaluation.merge(item_evaluation);
        if item.is_policy_exception {
            evaluation.warnings.push(format!(
                "Expense item {} marked as a policy exception",
                item.id
            ));
        }
    }

    evaluation
}

fn map_sqlx_error(err: sqlx::Error) -> ServiceError {
    ServiceError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use uuid::Uuid;

    use crate::{
        domain::models::Role,
        infrastructure::{
            auth::AuthenticatedUser,
            config::{
                AppConfig, AuthConfig, Config, DatabaseConfig, NetSuiteConfig, ReceiptRules,
                StorageConfig,
            },
            state::AppState,
            storage,
        },
    };

    fn expense_item(
        id: Uuid,
        date: NaiveDate,
        amount_cents: i64,
        is_exception: bool,
    ) -> ExpenseItem {
        ExpenseItem {
            id,
            report_id: Uuid::new_v4(),
            expense_date: date,
            category: ExpenseCategory::Meal,
            gl_account_id: None,
            description: Some("Test item".to_string()),
            attendees: None,
            location: None,
            amount_cents,
            reimbursable: true,
            payment_method: None,
            is_policy_exception: is_exception,
        }
    }

    fn meal_cap(amount_cents: i64, active_from: NaiveDate) -> PolicyCap {
        PolicyCap {
            id: Uuid::new_v4(),
            policy_key: "meal_per_diem".to_string(),
            category: ExpenseCategory::Meal,
            limit_type: "per_diem".to_string(),
            amount_cents,
            notes: None,
            active_from,
            active_to: None,
        }
    }

    #[test]
    fn aggregate_policy_evaluation_passes_for_compliant_items() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let caps = vec![meal_cap(5_000, date)];
        let items = vec![expense_item(Uuid::new_v4(), date, 4_000, false)];

        let evaluation = aggregate_policy_evaluation(&items, &caps);

        assert!(evaluation.is_valid);
        assert!(evaluation.violations.is_empty());
        assert!(evaluation.warnings.is_empty());
    }

    #[test]
    fn aggregate_policy_evaluation_flags_violations_and_warnings() {
        let date = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        let caps = vec![meal_cap(5_000, date)];
        let item_id = Uuid::new_v4();
        let items = vec![expense_item(item_id, date, 7_500, true)];

        let evaluation = aggregate_policy_evaluation(&items, &caps);

        assert!(!evaluation.is_valid);
        assert!(evaluation
            .violations
            .iter()
            .any(|msg| msg.contains("Meal exceeds per-diem limit")));
        assert_eq!(evaluation.warnings.len(), 1);
        assert!(evaluation.warnings[0].contains(item_id.to_string().as_str()));
    }

    #[test]
    fn calculate_totals_splits_reimbursable_amounts() {
        let date = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
        let items = vec![
            CreateExpenseItem {
                expense_date: date,
                category: ExpenseCategory::Meal,
                description: None,
                attendees: None,
                location: None,
                amount_cents: 2_500,
                reimbursable: true,
                payment_method: None,
                receipts: Vec::new(),
            },
            CreateExpenseItem {
                expense_date: date,
                category: ExpenseCategory::Lodging,
                description: None,
                attendees: None,
                location: None,
                amount_cents: 7_500,
                reimbursable: false,
                payment_method: None,
                receipts: Vec::new(),
            },
        ];

        let (total, reimbursable) = calculate_totals(&items);

        assert_eq!(total, 10_000);
        assert_eq!(reimbursable, 2_500);
    }

    #[tokio::test]
    async fn create_report_persists_items_and_receipts() -> anyhow::Result<()> {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("EXPENSES__DATABASE__URL"))
            .unwrap_or_else(|_| "postgres://expenses:expenses@localhost:5432/expenses".to_string());

        let pool = match PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
        {
            Ok(pool) => pool,
            Err(err) => {
                eprintln!("Skipping create_report_persists_items_and_receipts test: {err}");
                return Ok(());
            }
        };

        sqlx::migrate!("./migrations").run(&pool).await?;

        run_create_report_scenario(pool).await
    }

    async fn run_create_report_scenario(pool: PgPool) -> anyhow::Result<()> {
        let employee_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(employee_id)
        .bind(format!("EMP{}", Uuid::new_v4().simple()))
        .bind::<Option<Uuid>>(None)
        .bind::<Option<String>>(None)
        .bind(Role::Employee)
        .bind(chrono::Utc::now())
        .execute(&pool)
        .await?;

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
        let service = ExpenseService::new(Arc::clone(&state));
        let actor = AuthenticatedUser {
            employee_id,
            role: Role::Employee,
        };

        let reporting_period_start = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
        let reporting_period_end = NaiveDate::from_ymd_opt(2024, 5, 31).unwrap();
        let payload = CreateReportRequest {
            reporting_period_start,
            reporting_period_end,
            currency: "USD".to_string(),
            items: vec![
                CreateExpenseItem {
                    expense_date: reporting_period_start,
                    category: ExpenseCategory::Meal,
                    description: Some("Team kickoff lunch".to_string()),
                    attendees: Some("S. Mills; A. Chen".to_string()),
                    location: Some("Portland".to_string()),
                    amount_cents: 4_200,
                    reimbursable: true,
                    payment_method: Some("corporate_card".to_string()),
                    receipts: vec![CreateReceiptReference {
                        file_key: "draft-receipt-1".to_string(),
                        file_name: "lunch.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 32_000,
                    }],
                },
                CreateExpenseItem {
                    expense_date: reporting_period_start,
                    category: ExpenseCategory::Lodging,
                    description: Some("Client site lodging".to_string()),
                    attendees: None,
                    location: Some("Portland".to_string()),
                    amount_cents: 18_500,
                    reimbursable: false,
                    payment_method: Some("personal_card".to_string()),
                    receipts: Vec::new(),
                },
            ],
        };

        let report = service.create_report(&actor, payload).await?;

        let stored_items = sqlx::query(
            "SELECT amount_cents, reimbursable FROM expense_items WHERE report_id = $1",
        )
        .bind(report.id)
        .fetch_all(&pool)
        .await?;

        assert_eq!(stored_items.len(), 2);
        assert!(stored_items.iter().any(|row| {
            row.get::<bool, _>("reimbursable") && row.get::<i64, _>("amount_cents") == 4_200
        }));
        assert!(stored_items.iter().any(|row| {
            !row.get::<bool, _>("reimbursable") && row.get::<i64, _>("amount_cents") == 18_500
        }));

        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM receipts r JOIN expense_items i ON r.expense_item_id = i.id WHERE i.report_id = $1",
        )
        .bind(report.id)
        .fetch_one(&pool)
        .await?;

        assert_eq!(receipt_count, 1);
        assert_eq!(report.total_amount_cents, 22_700);
        assert_eq!(report.total_reimbursable_cents, 4_200);

        sqlx::query("DELETE FROM expense_reports WHERE id = $1")
            .bind(report.id)
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM employees WHERE id = $1")
            .bind(employee_id)
            .execute(&pool)
            .await?;

        Ok(())
    }
}
