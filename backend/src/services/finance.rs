use std::sync::Arc;

use chrono::Utc;
use serde::Deserialize;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

use crate::{
    domain::models::{JournalLine, NetSuiteBatch, ReportStatus, Role},
    infrastructure::{auth::AuthenticatedUser, netsuite, state::AppState},
};

use super::errors::ServiceError;

#[derive(Debug, Deserialize)]
pub struct FinalizeRequest {
    pub report_ids: Vec<Uuid>,
    pub batch_reference: String,
}

pub struct FinanceService {
    pub state: Arc<AppState>,
}

impl FinanceService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn finalize_reports(
        &self,
        actor: &AuthenticatedUser,
        payload: FinalizeRequest,
    ) -> Result<NetSuiteBatch, ServiceError> {
        if actor.role != Role::Finance {
            return Err(ServiceError::Forbidden);
        }
        let batch = sqlx::query(
            "INSERT INTO netsuite_batches (id, batch_reference, finalized_by, finalized_at, status)
             VALUES ($1,$2,$3,$4,$5) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(&payload.batch_reference)
        .bind(actor.employee_id)
        .bind(Utc::now())
        .bind("pending")
        .map(|row: PgRow| map_batch(row))
        .fetch_one(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        let mut lines = Vec::new();
        for (idx, report_id) in payload.report_ids.iter().enumerate() {
            sqlx::query("UPDATE expense_reports SET status=$1 WHERE id=$2")
                .bind(ReportStatus::FinanceFinalized.as_str())
                .bind(report_id)
                .execute(&self.state.pool)
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
            .fetch_one(&self.state.pool)
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;
            lines.push(line);
        }

        let response = netsuite::export_batch(&batch, &lines)
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;
        sqlx::query("UPDATE netsuite_batches SET status=$1, exported_at=$2, netsuite_response=$3 WHERE id=$4")
            .bind(if response.succeeded { "exported" } else { "failed" })
            .bind(Utc::now())
            .bind(serde_json::to_value(&response).ok())
            .bind(batch.id)
            .execute(&self.state.pool)
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
