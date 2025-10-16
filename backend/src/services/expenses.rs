use std::{collections::HashSet, sync::Arc};

use chrono::Utc;
use serde::Deserialize;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

use crate::{
    domain::{
        models::{ExpenseCategory, ExpenseItem, ExpenseReport, PolicyCap, ReportStatus},
        policy::{evaluate_item, PolicyEvaluation},
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
            "UPDATE expense_reports SET status=$1, version=version+1, updated_at=$2 WHERE id=$3 AND employee_id=$4 AND status='draft' RETURNING *",
        )
        .bind(ReportStatus::Submitted.as_str())
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

        let mut category_keys: HashSet<String> = HashSet::new();
        for item in &items {
            category_keys.insert(item.category.as_str().to_string());
        }
        let categories: Vec<String> = category_keys.into_iter().collect();

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

fn map_expense_item(row: PgRow) -> Result<ExpenseItem, ServiceError> {
    let category = row
        .try_get::<String, _>("category")
        .map_err(map_sqlx_error)?
        .parse::<ExpenseCategory>()
        .map_err(ServiceError::Internal)?;
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
        .try_get::<String, _>("category")
        .map_err(map_sqlx_error)?
        .parse::<ExpenseCategory>()
        .map_err(ServiceError::Internal)?;
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
    use uuid::Uuid;

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
}
