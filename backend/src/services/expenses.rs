use std::sync::Arc;

use chrono::Utc;
use serde::Deserialize;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

use crate::{
    domain::{
        models::{ExpenseReport, ReportStatus},
        policy::PolicyEvaluation,
    },
    infrastructure::state::AppState,
};

use super::errors::ServiceError;

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub reporting_period_start: chrono::NaiveDate,
    pub reporting_period_end: chrono::NaiveDate,
    pub currency: String,
}

pub struct ExpenseService {
    pub state: Arc<AppState>,
}

impl ExpenseService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn create_report(
        &self,
        actor: &crate::infrastructure::auth::AuthenticatedUser,
        payload: CreateReportRequest,
    ) -> Result<ExpenseReport, ServiceError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status = ReportStatus::Draft;
        let record = sqlx::query(
            "INSERT INTO expense_reports (id, employee_id, reporting_period_start, reporting_period_end, status, total_amount_cents, total_reimbursable_cents, currency, version, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             RETURNING *",
        )
        .bind(id)
        .bind(actor.employee_id)
        .bind(payload.reporting_period_start)
        .bind(payload.reporting_period_end)
        .bind(status.as_str())
        .bind(0_i64)
        .bind(0_i64)
        .bind(payload.currency)
        .bind(1_i32)
        .bind(now)
        .bind(now)
        .map(|row: PgRow| map_report(row))
        .fetch_one(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;
        Ok(record)
    }

    pub async fn submit_report(
        &self,
        actor: &crate::infrastructure::auth::AuthenticatedUser,
        report_id: Uuid,
    ) -> Result<ExpenseReport, ServiceError> {
        let record = sqlx::query(
            "UPDATE expense_reports SET status=$1, version=version+1, updated_at=$2 WHERE id=$3 AND employee_id=$4 RETURNING *",
        )
        .bind(ReportStatus::Submitted.as_str())
        .bind(Utc::now())
        .bind(report_id)
        .bind(actor.employee_id)
        .map(|row: PgRow| map_report(row))
        .fetch_optional(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;
        record.ok_or(ServiceError::NotFound)
    }

    pub async fn evaluate_report(&self, report_id: Uuid) -> Result<PolicyEvaluation, ServiceError> {
        let exists =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM expense_reports WHERE id = $1")
                .bind(report_id)
                .fetch_one(&self.state.pool)
                .await
                .map_err(|err| ServiceError::Internal(err.to_string()))?;
        if exists == 0 {
            return Err(ServiceError::NotFound);
        }
        Ok(PolicyEvaluation::ok())
    }
}

fn map_report(row: PgRow) -> ExpenseReport {
    ExpenseReport {
        id: row.get("id"),
        employee_id: row.get("employee_id"),
        reporting_period_start: row.get("reporting_period_start"),
        reporting_period_end: row.get("reporting_period_end"),
        status: row
            .get::<String, _>("status")
            .parse::<ReportStatus>()
            .unwrap_or(ReportStatus::Draft),
        total_amount_cents: row.get("total_amount_cents"),
        total_reimbursable_cents: row.get("total_reimbursable_cents"),
        currency: row.get("currency"),
        version: row.get("version"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
