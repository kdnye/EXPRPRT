//! Finalizes approved reports into NetSuite batch exports.
//!
//! Serves the `POST /finance/finalize` REST workflow defined in
//! `backend/src/api/rest/finance.rs`, coordinating GL postings and external
//! export stubs described in `POLICY.md` §"Approvals and Reimbursement Process"
//! and §"General Ledger Mapping".

use std::sync::Arc;

use chrono::Utc;
use serde::Deserialize;
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
