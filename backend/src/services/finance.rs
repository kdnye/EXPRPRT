//! Finalizes approved reports into NetSuite batch exports.
//!
//! Serves the `POST /finance/finalize` REST workflow defined in
//! `backend/src/api/rest/finance.rs`, coordinating GL postings and external
//! export stubs described in `POLICY.md` §"Approvals and Reimbursement Process"
//! and §"General Ledger Mapping".

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    domain::models::{JournalLine, NetSuiteBatch, ReportStatus, Role},
    infrastructure::{auth::AuthenticatedUser, netsuite, state::AppState},
};

use super::errors::ServiceError;

/// Payload accepted by `POST /finance/finalize` containing the reports to post
/// and the NetSuite batch metadata.
///
/// Report identifiers should correspond to records already marked
/// `ReportStatus::FinanceFinalized` by the approval workflow outlined in
/// `POLICY.md` §"Approvals and Reimbursement Process".
#[derive(Debug, Deserialize)]
pub struct FinalizeRequest {
    pub report_ids: Vec<Uuid>,
    pub batch_reference: String,
}

/// Coordinates journal line creation and NetSuite export invocations.
pub struct FinanceService {
    pub state: Arc<AppState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchSummary {
    pub id: Uuid,
    pub batch_reference: String,
    pub finalized_at: DateTime<Utc>,
    pub status: String,
    pub exported_at: Option<DateTime<Utc>>,
    pub report_count: i64,
    pub total_amount_cents: i64,
}

impl FinanceService {
    /// Constructs the finance integration service from shared application
    /// state.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Finalizes a batch of reports by persisting GL lines and invoking the
    /// NetSuite export adapter.
    ///
    /// * `actor` — authenticated finance user; must have the `Role::Finance`
    ///   designation to comply with segregation of duties in `POLICY.md`
    ///   §"Approvals and Reimbursement Process".
    /// * `payload` — report identifiers and reference string consumed by
    ///   downstream accounting processes.
    ///
    /// Side effects:
    /// * Creates a `NetSuiteBatch` record and related `JournalLine` entries,
    ///   populating GL accounts described in `POLICY.md` §"General Ledger
    ///   Mapping".
    /// * Calls `infrastructure::netsuite::export_batch`, a stubbed integration
    ///   point for NetSuite, and stores the serialized response.
    /// * Updates each report status to `ReportStatus::FinanceFinalized` to signal
    ///   completion back to the approvals domain.
    pub async fn finalize_reports(
        &self,
        actor: &AuthenticatedUser,
        payload: FinalizeRequest,
    ) -> Result<NetSuiteBatch, ServiceError> {
        if actor.role != Role::Finance {
            return Err(ServiceError::Forbidden);
        }
        let mut tx: Transaction<'_, Postgres> = self
            .state
            .pool
            .begin()
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;

        let mut batch = sqlx::query(
            "INSERT INTO netsuite_batches (id, batch_reference, finalized_by, finalized_at, status)
             VALUES ($1,$2,$3,$4,$5) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(&payload.batch_reference)
        .bind(actor.employee_id)
        .bind(Utc::now())
        .bind("pending")
        .map(|row: PgRow| map_batch(row))
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        let mut lines = Vec::new();
        for (idx, report_id) in payload.report_ids.iter().enumerate() {
            sqlx::query("UPDATE expense_reports SET status=$1 WHERE id=$2")
                .bind(ReportStatus::FinanceFinalized)
                .bind(report_id)
                .execute(tx.as_mut())
                .await
                .map_err(|err| ServiceError::Internal(err.to_string()))?;
            let line = sqlx::query(
                "INSERT INTO journal_lines (id, batch_id, report_id, line_number, gl_account, amount_cents)
                 VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
            )
            .bind(Uuid::new_v4())
            .bind(batch.id)
            .bind(report_id)
            .bind((idx + 1) as i32)
            .bind("EXPENSES")
            .bind(0_i64)
            .map(|row: PgRow| map_line(row))
            .fetch_one(tx.as_mut())
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;
            lines.push(line);
        }

        let response = match netsuite::export_batch(&batch, &lines).await {
            Ok(response) => response,
            Err(err) => {
                if let Err(rollback_err) = tx.rollback().await {
                    return Err(ServiceError::Internal(format!(
                        "failed to rollback after NetSuite export error: {} (original: {})",
                        rollback_err, err
                    )));
                }
                return Err(ServiceError::Internal(err.to_string()));
            }
        };

        let export_status = if response.succeeded {
            "exported"
        } else {
            "failed"
        };
        let exported_at = Utc::now();
        let response_json = serde_json::to_value(&response).ok();

        sqlx::query(
            "UPDATE netsuite_batches SET status=$1, exported_at=$2, netsuite_response=$3 WHERE id=$4",
        )
        .bind(export_status)
        .bind(exported_at)
        .bind(response_json.clone())
        .bind(batch.id)
        .execute(tx.as_mut())
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        batch.status = export_status.to_string();
        batch.exported_at = Some(exported_at);
        batch.netsuite_response = response_json;

        tx.commit()
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;

        Ok(batch)
    }

    /// Returns recent NetSuite batches with aggregate journal statistics for
    /// finance visibility.
    pub async fn recent_batches(
        &self,
        actor: &AuthenticatedUser,
    ) -> Result<Vec<BatchSummary>, ServiceError> {
        if actor.role != Role::Finance {
            return Err(ServiceError::Forbidden);
        }

        const LIMIT: i64 = 25;
        let batches = sqlx::query(
            "SELECT b.id, b.batch_reference, b.finalized_at, b.status, b.exported_at,
                    COUNT(DISTINCT j.report_id) AS report_count,
                    COALESCE(SUM(j.amount_cents), 0) AS total_amount_cents
             FROM netsuite_batches b
             LEFT JOIN journal_lines j ON j.batch_id = b.id
             GROUP BY b.id
             ORDER BY b.finalized_at DESC
             LIMIT $1",
        )
        .bind(LIMIT)
        .map(|row: PgRow| BatchSummary {
            id: row.get("id"),
            batch_reference: row.get("batch_reference"),
            finalized_at: row.get("finalized_at"),
            status: row.get("status"),
            exported_at: row.get("exported_at"),
            report_count: row.get::<i64, _>("report_count"),
            total_amount_cents: row.get::<i64, _>("total_amount_cents"),
        })
        .fetch_all(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        Ok(batches)
    }
}

fn map_batch(row: PgRow) -> NetSuiteBatch {
    NetSuiteBatch {
        id: row.get("id"),
        batch_reference: row.get("batch_reference"),
        finalized_by: row.get("finalized_by"),
        finalized_at: row.get("finalized_at"),
        status: row.get("status"),
        exported_at: row.get("exported_at"),
        netsuite_response: row.get("netsuite_response"),
    }
}

fn map_line(row: PgRow) -> JournalLine {
    JournalLine {
        id: row.get("id"),
        batch_id: row.get("batch_id"),
        report_id: row.get("report_id"),
        line_number: row.get("line_number"),
        gl_account: row.get("gl_account"),
        amount_cents: row.get("amount_cents"),
        department: row.get("department"),
        class: row.get("class"),
        memo: row.get("memo"),
        tax_code: row.get("tax_code"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use chrono::{Duration, NaiveDate};
    use sqlx::{postgres::PgPoolOptions, PgPool};

    use crate::{
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

    #[tokio::test]
    async fn recent_batches_returns_empty_when_none_exist() -> Result<()> {
        let Some((state, pool)) = setup_state().await? else {
            return Ok(());
        };

        sqlx::query("DELETE FROM journal_lines")
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM netsuite_batches")
            .execute(&pool)
            .await?;

        let service = FinanceService::new(state);
        let actor = AuthenticatedUser {
            employee_id: Uuid::new_v4(),
            role: Role::Finance,
        };

        let batches = service.recent_batches(&actor).await?;
        assert!(batches.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn recent_batches_returns_populated_summary() -> Result<()> {
        let Some((state, pool)) = setup_state().await? else {
            return Ok(());
        };

        sqlx::query("DELETE FROM journal_lines")
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM netsuite_batches")
            .execute(&pool)
            .await?;

        let finance_employee = Uuid::new_v4();
        let hr_identifier = format!("FIN-{}", finance_employee.simple());
        sqlx::query(
            "INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(finance_employee)
        .bind(&hr_identifier)
        .bind::<Option<Uuid>>(None)
        .bind::<Option<String>>(Some("Finance".to_string()))
        .bind(Role::Finance)
        .bind(Utc::now())
        .execute(&pool)
        .await?;

        let period_start = NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date");
        let period_end = NaiveDate::from_ymd_opt(2024, 5, 31).expect("valid date");

        let report_a = Uuid::new_v4();
        let report_b = Uuid::new_v4();
        let report_c = Uuid::new_v4();

        for report_id in [report_a, report_b, report_c] {
            sqlx::query(
                "INSERT INTO expense_reports
                     (id, employee_id, reporting_period_start, reporting_period_end, status,
                      total_amount_cents, total_reimbursable_cents, currency, version, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            )
            .bind(report_id)
            .bind(finance_employee)
            .bind(period_start)
            .bind(period_end)
            .bind("finance_finalized")
            .bind(10_000_i64)
            .bind(10_000_i64)
            .bind("USD")
            .bind(1_i32)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&pool)
            .await?;
        }

        let older_batch = Uuid::new_v4();
        let recent_batch = Uuid::new_v4();
        let older_finalized = Utc::now() - Duration::days(2);
        let recent_finalized = Utc::now() - Duration::hours(12);

        sqlx::query(
            "INSERT INTO netsuite_batches (id, batch_reference, finalized_by, finalized_at, status, exported_at, netsuite_response)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(older_batch)
        .bind("APR-2024-01")
        .bind(finance_employee)
        .bind(older_finalized)
        .bind("pending")
        .bind::<Option<chrono::DateTime<Utc>>>(None)
        .bind::<Option<serde_json::Value>>(None)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO netsuite_batches (id, batch_reference, finalized_by, finalized_at, status, exported_at, netsuite_response)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(recent_batch)
        .bind("APR-2024-02")
        .bind(finance_employee)
        .bind(recent_finalized)
        .bind("exported")
        .bind::<Option<chrono::DateTime<Utc>>>(Some(Utc::now()))
        .bind::<Option<serde_json::Value>>(Some(serde_json::json!({"status": "ok"})))
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO journal_lines (id, batch_id, report_id, line_number, gl_account, amount_cents, department, class, memo, tax_code)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(Uuid::new_v4())
        .bind(older_batch)
        .bind(report_a)
        .bind(1_i32)
        .bind("EXPENSES")
        .bind(42_500_i64)
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(Some("Travel".to_string()))
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO journal_lines (id, batch_id, report_id, line_number, gl_account, amount_cents, department, class, memo, tax_code)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(Uuid::new_v4())
        .bind(recent_batch)
        .bind(report_b)
        .bind(1_i32)
        .bind("EXPENSES")
        .bind(30_000_i64)
        .bind::<Option<String>>(Some("Ops".to_string()))
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(Some("Meals".to_string()))
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO journal_lines (id, batch_id, report_id, line_number, gl_account, amount_cents, department, class, memo, tax_code)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(Uuid::new_v4())
        .bind(recent_batch)
        .bind(report_b)
        .bind(2_i32)
        .bind("EXPENSES")
        .bind(12_500_i64)
        .bind::<Option<String>>(Some("Ops".to_string()))
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(Some("Lodging".to_string()))
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO journal_lines (id, batch_id, report_id, line_number, gl_account, amount_cents, department, class, memo, tax_code)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(Uuid::new_v4())
        .bind(recent_batch)
        .bind(report_c)
        .bind(3_i32)
        .bind("EXPENSES")
        .bind(15_000_i64)
        .bind::<Option<String>>(Some("Ops".to_string()))
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(Some("Airfare".to_string()))
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await?;

        let service = FinanceService::new(Arc::clone(&state));
        let actor = AuthenticatedUser {
            employee_id: finance_employee,
            role: Role::Finance,
        };

        let batches = service.recent_batches(&actor).await?;
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].id, recent_batch);
        assert_eq!(batches[0].status, "exported");
        assert!(batches[0].exported_at.is_some());
        assert_eq!(batches[0].finalized_at, recent_finalized);
        assert_eq!(batches[0].report_count, 2);
        assert_eq!(batches[0].total_amount_cents, 57_500_i64);
        assert_eq!(batches[1].id, older_batch);
        assert_eq!(batches[1].status, "pending");
        assert!(batches[1].exported_at.is_none());
        assert_eq!(batches[1].finalized_at, older_finalized);
        assert_eq!(batches[1].report_count, 1);
        assert_eq!(batches[1].total_amount_cents, 42_500_i64);

        sqlx::query("DELETE FROM netsuite_batches WHERE id = ANY($1)")
            .bind(&vec![older_batch, recent_batch])
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM expense_reports WHERE id = ANY($1)")
            .bind(&vec![report_a, report_b, report_c])
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM employees WHERE id = $1")
            .bind(finance_employee)
            .execute(&pool)
            .await?;

        Ok(())
    }

    async fn setup_state() -> Result<Option<(Arc<AppState>, PgPool)>> {
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
                eprintln!("Skipping finance service tests: unable to connect to database: {err}");
                return Ok(None);
            }
        };

        sqlx::migrate!("./migrations").run(&pool).await?;

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

        Ok(Some((state, pool)))
    }
}
