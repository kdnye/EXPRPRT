use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::FromRow;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    Employee,
    Manager,
    Finance,
    Admin,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Employee => "employee",
            Role::Manager => "manager",
            Role::Finance => "finance",
            Role::Admin => "admin",
        }
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "employee" => Ok(Role::Employee),
            "manager" => Ok(Role::Manager),
            "finance" => Ok(Role::Finance),
            "admin" => Ok(Role::Admin),
            other => Err(format!("unknown role {other}")),
        }
    }
}

impl From<Role> for String {
    fn from(role: Role) -> Self {
        role.as_str().to_string()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Employee {
    pub id: Uuid,
    pub hr_identifier: String,
    pub manager_id: Option<Uuid>,
    pub department: Option<String>,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportStatus {
    Draft,
    Submitted,
    ManagerApproved,
    FinanceFinalized,
    NeedsChanges,
    Denied,
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportStatus::Draft => "draft",
            ReportStatus::Submitted => "submitted",
            ReportStatus::ManagerApproved => "manager_approved",
            ReportStatus::FinanceFinalized => "finance_finalized",
            ReportStatus::NeedsChanges => "needs_changes",
            ReportStatus::Denied => "denied",
        }
    }
}

impl FromStr for ReportStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(ReportStatus::Draft),
            "submitted" => Ok(ReportStatus::Submitted),
            "manager_approved" => Ok(ReportStatus::ManagerApproved),
            "finance_finalized" => Ok(ReportStatus::FinanceFinalized),
            "needs_changes" => Ok(ReportStatus::NeedsChanges),
            "denied" => Ok(ReportStatus::Denied),
            other => Err(format!("unknown report status {other}")),
        }
    }
}

impl From<ReportStatus> for String {
    fn from(status: ReportStatus) -> Self {
        status.as_str().to_string()
    }
}
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExpenseReport {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub reporting_period_start: NaiveDate,
    pub reporting_period_end: NaiveDate,
    pub status: ReportStatus,
    pub total_amount_cents: i64,
    pub total_reimbursable_cents: i64,
    pub currency: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpenseCategory {
    Airfare,
    Lodging,
    Meal,
    GroundTransport,
    Mileage,
    Supplies,
    Other,
}

impl ExpenseCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpenseCategory::Airfare => "airfare",
            ExpenseCategory::Lodging => "lodging",
            ExpenseCategory::Meal => "meal",
            ExpenseCategory::GroundTransport => "ground_transport",
            ExpenseCategory::Mileage => "mileage",
            ExpenseCategory::Supplies => "supplies",
            ExpenseCategory::Other => "other",
        }
    }
}

impl FromStr for ExpenseCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "airfare" => Ok(ExpenseCategory::Airfare),
            "lodging" => Ok(ExpenseCategory::Lodging),
            "meal" => Ok(ExpenseCategory::Meal),
            "ground_transport" => Ok(ExpenseCategory::GroundTransport),
            "mileage" => Ok(ExpenseCategory::Mileage),
            "supplies" => Ok(ExpenseCategory::Supplies),
            "other" => Ok(ExpenseCategory::Other),
            other => Err(format!("unknown expense category {other}")),
        }
    }
}

impl From<ExpenseCategory> for String {
    fn from(category: ExpenseCategory) -> Self {
        category.as_str().to_string()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExpenseItem {
    pub id: Uuid,
    pub report_id: Uuid,
    pub expense_date: NaiveDate,
    pub category: ExpenseCategory,
    pub gl_account_id: Option<Uuid>,
    pub description: Option<String>,
    pub attendees: Option<String>,
    pub location: Option<String>,
    pub amount_cents: i64,
    pub reimbursable: bool,
    pub payment_method: Option<String>,
    pub is_policy_exception: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Receipt {
    pub id: Uuid,
    pub expense_item_id: Uuid,
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalStatus {
    Approved,
    Denied,
    NeedsChanges,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Denied => "denied",
            ApprovalStatus::NeedsChanges => "needs_changes",
        }
    }
}

impl FromStr for ApprovalStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "approved" => Ok(ApprovalStatus::Approved),
            "denied" => Ok(ApprovalStatus::Denied),
            "needs_changes" => Ok(ApprovalStatus::NeedsChanges),
            other => Err(format!("unknown approval status {other}")),
        }
    }
}

impl From<ApprovalStatus> for String {
    fn from(status: ApprovalStatus) -> Self {
        status.as_str().to_string()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Approval {
    pub id: Uuid,
    pub report_id: Uuid,
    pub approver_id: Uuid,
    pub role: Role,
    pub status: ApprovalStatus,
    pub comments: Option<String>,
    pub policy_exception_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NetSuiteBatch {
    pub id: Uuid,
    pub batch_reference: String,
    pub finalized_by: Uuid,
    pub finalized_at: DateTime<Utc>,
    pub status: String,
    pub exported_at: Option<DateTime<Utc>>,
    pub netsuite_response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JournalLine {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub report_id: Uuid,
    pub line_number: i32,
    pub gl_account: String,
    pub amount_cents: i64,
    pub department: Option<String>,
    pub class: Option<String>,
    pub memo: Option<String>,
    pub tax_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MileageRate {
    pub effective_date: NaiveDate,
    pub rate_cents_per_mile: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PolicyCap {
    pub id: Uuid,
    pub policy_key: String,
    pub category: ExpenseCategory,
    pub limit_type: String,
    pub amount_cents: i64,
    pub notes: Option<String>,
    pub active_from: NaiveDate,
    pub active_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub event_type: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub performed_by: Option<Uuid>,
    pub performed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub signature_hash: String,
}
