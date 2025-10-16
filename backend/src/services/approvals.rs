//! Manages managerial and finance approval decisions for expense reports.
//!
//! Backing service for the `POST /approvals/:id` route in
//! `backend/src/api/rest/approvals.rs`, ensuring role-based transitions mirror
//! the governance spelled out in `POLICY.md` §"Approvals and Reimbursement
//! Process".

use std::sync::Arc;

use chrono::Utc;
use serde::Deserialize;
use sqlx::{postgres::PgRow, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    domain::models::{Approval, ApprovalStatus, ReportStatus, Role},
    infrastructure::{auth::AuthenticatedUser, state::AppState},
};

use super::errors::ServiceError;

/// Manager or finance decision recorded through `POST /approvals/:id`.
///
/// Includes optional `policy_exception_notes` so reviewers can document why an
/// override aligns with the escalation paths in `POLICY.md`
/// §"Approvals and Reimbursement Process".
#[derive(Debug, Deserialize)]
pub struct DecisionRequest {
    pub status: ApprovalStatus,
    pub comments: Option<String>,
    pub policy_exception_notes: Option<String>,
}

/// Service coordinating approval persistence and report status transitions.
pub struct ApprovalService {
    pub state: Arc<AppState>,
}

impl ApprovalService {
    /// Constructs the service using the shared database connection pool.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Records a manager or finance decision and transitions report state when
    /// appropriate.
    ///
    /// * `actor` — authenticated approver; role is validated against
    ///   `Role::Manager` or `Role::Finance` per `POLICY.md`
    ///   §"Approvals and Reimbursement Process".
    /// * `report_id` — target report awaiting action.
    /// * `payload` — desired decision, optional comments, and policy exception
    ///   rationale for audit.
    ///
    /// Side effects:
    /// * Persists an `Approval` row and ensures history capture.
    /// * Promotes report status to `ReportStatus::ManagerApproved` or
    ///   `ReportStatus::FinanceFinalized`, coordinating hand-offs to the
    ///   finance export pipeline implemented in `FinanceService`.
    ///
    /// Fails with `ServiceError::Forbidden` when the actor's role is outside of
    /// the allowed reviewers, leveraging the same `Role` model used elsewhere
    /// in the domain.
    pub async fn record_decision(
        &self,
        actor: &AuthenticatedUser,
        report_id: Uuid,
        payload: DecisionRequest,
    ) -> Result<Approval, ServiceError> {
        ensure_role(actor, &[Role::Manager, Role::Finance])?;
        let mut tx = self
            .state
            .pool
            .begin()
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;
        let now = Utc::now();
        let approval = sqlx::query(
            "INSERT INTO approvals (id, report_id, approver_id, role, status, comments, policy_exception_notes, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(report_id)
        .bind(actor.employee_id)
        .bind(actor.role)
        .bind(payload.status)
        .bind(payload.comments)
        .bind(payload.policy_exception_notes)
        .bind(now)
        .map(|row: PgRow| map_approval(row))
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| ServiceError::Internal(err.to_string()))?;

        if actor.role == Role::Manager && payload.status == ApprovalStatus::Approved {
            self.transition_report(&mut tx, report_id, ReportStatus::ManagerApproved)
                .await?;
        }
        if actor.role == Role::Finance && payload.status == ApprovalStatus::Approved {
            self.transition_report(&mut tx, report_id, ReportStatus::FinanceFinalized)
                .await?;
        }
        tx.commit()
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;
        Ok(approval)
    }

    async fn transition_report(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        report_id: Uuid,
        status: ReportStatus,
    ) -> Result<(), ServiceError> {
        let result = sqlx::query("UPDATE expense_reports SET status=$1, updated_at=$2 WHERE id=$3")
            .bind(status)
            .bind(Utc::now())
            .bind(report_id)
            .execute(tx.as_mut())
            .await
            .map_err(|err| ServiceError::Internal(err.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }
}

fn ensure_role(user: &AuthenticatedUser, allowed: &[Role]) -> Result<(), ServiceError> {
    if allowed.iter().any(|r| r == &user.role) {
        Ok(())
    } else {
        Err(ServiceError::Forbidden)
    }
}

fn map_approval(row: PgRow) -> Approval {
    Approval {
        id: row.get("id"),
        report_id: row.get("report_id"),
        approver_id: row.get("approver_id"),
        role: row.get("role"),
        status: row.get("status"),
        comments: row.get("comments"),
        policy_exception_notes: row.get("policy_exception_notes"),
        created_at: row.get("created_at"),
    }
}
