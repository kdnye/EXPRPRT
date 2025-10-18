use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    domain::models::{ReportStatus, Role},
    infrastructure::{auth::AuthenticatedUser, state::AppState},
};

use super::errors::ServiceError;

/// Service exposing manager-focused aggregates for pending expense reports.
pub struct ManagerService {
    state: Arc<AppState>,
}

impl ManagerService {
    /// Constructs the service from shared application state.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Returns the queue of submitted expense reports awaiting manager review.
    ///
    /// Only actors with the `Role::Manager` designation may access the queue.
    pub async fn fetch_queue(
        &self,
        actor: &AuthenticatedUser,
    ) -> Result<Vec<ManagerQueueEntry>, ServiceError> {
        if actor.role != Role::Manager {
            return Err(ServiceError::Forbidden);
        }

        let reports: Vec<ReportRow> = sqlx::query_as(
            r#"
            SELECT
                r.id,
                r.employee_id,
                e.hr_identifier,
                r.reporting_period_start,
                r.reporting_period_end,
                r.total_amount_cents,
                r.total_reimbursable_cents,
                r.currency,
                r.updated_at AS submitted_at
            FROM expense_reports r
            JOIN employees e ON e.id = r.employee_id
            WHERE r.status = $1
            ORDER BY submitted_at ASC, r.id ASC
            "#,
        )
        .bind(ReportStatus::Submitted.as_str())
        .fetch_all(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        if reports.is_empty() {
            return Ok(Vec::new());
        }

        let report_ids: Vec<Uuid> = reports.iter().map(|report| report.id).collect();

        let items: Vec<ItemRow> = sqlx::query_as(
            r#"
            SELECT
                id,
                report_id,
                expense_date,
                category,
                description,
                amount_cents,
                reimbursable,
                payment_method,
                is_policy_exception
            FROM expense_items
            WHERE report_id = ANY($1)
            ORDER BY expense_date ASC, id ASC
            "#,
        )
        .bind(&report_ids)
        .fetch_all(&self.state.pool)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        let mut items_by_report: HashMap<Uuid, Vec<ManagerQueueLineItem>> = HashMap::new();
        for item in items {
            let entry = ManagerQueueLineItem {
                id: item.id,
                report_id: item.report_id,
                expense_date: item.expense_date,
                category: item.category,
                description: item.description,
                amount_cents: item.amount_cents,
                reimbursable: item.reimbursable,
                payment_method: item.payment_method,
                is_policy_exception: item.is_policy_exception,
            };
            items_by_report
                .entry(entry.report_id)
                .or_default()
                .push(entry);
        }

        let mut queue = Vec::with_capacity(reports.len());
        for report in reports {
            let items = items_by_report.remove(&report.id).unwrap_or_default();
            let policy_flags = items
                .iter()
                .filter(|item| item.is_policy_exception)
                .map(|item| ManagerPolicyFlag {
                    item_id: item.id,
                    category: item.category.clone(),
                    expense_date: item.expense_date,
                    description: item.description.clone(),
                })
                .collect();

            queue.push(ManagerQueueEntry {
                report: report.into(),
                line_items: items,
                policy_flags,
            });
        }

        Ok(queue)
    }
}

#[derive(Debug, FromRow)]
struct ReportRow {
    id: Uuid,
    employee_id: Uuid,
    hr_identifier: String,
    reporting_period_start: NaiveDate,
    reporting_period_end: NaiveDate,
    total_amount_cents: i64,
    total_reimbursable_cents: i64,
    currency: String,
    submitted_at: DateTime<Utc>,
}

impl From<ReportRow> for ManagerQueueReport {
    fn from(value: ReportRow) -> Self {
        Self {
            id: value.id,
            employee_id: value.employee_id,
            employee_hr_identifier: value.hr_identifier,
            reporting_period_start: value.reporting_period_start,
            reporting_period_end: value.reporting_period_end,
            submitted_at: value.submitted_at,
            total_amount_cents: value.total_amount_cents,
            total_reimbursable_cents: value.total_reimbursable_cents,
            currency: value.currency,
        }
    }
}

#[derive(Debug, FromRow)]
struct ItemRow {
    id: Uuid,
    report_id: Uuid,
    expense_date: NaiveDate,
    category: String,
    description: Option<String>,
    amount_cents: i64,
    reimbursable: bool,
    payment_method: Option<String>,
    is_policy_exception: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerQueueEntry {
    pub report: ManagerQueueReport,
    pub line_items: Vec<ManagerQueueLineItem>,
    pub policy_flags: Vec<ManagerPolicyFlag>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerQueueReport {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub employee_hr_identifier: String,
    pub reporting_period_start: NaiveDate,
    pub reporting_period_end: NaiveDate,
    pub submitted_at: DateTime<Utc>,
    pub total_amount_cents: i64,
    pub total_reimbursable_cents: i64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerQueueLineItem {
    pub id: Uuid,
    pub report_id: Uuid,
    pub expense_date: NaiveDate,
    pub category: String,
    pub description: Option<String>,
    pub amount_cents: i64,
    pub reimbursable: bool,
    pub payment_method: Option<String>,
    pub is_policy_exception: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerPolicyFlag {
    pub item_id: Uuid,
    pub category: String,
    pub expense_date: NaiveDate,
    pub description: Option<String>,
}
