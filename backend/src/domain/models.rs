use std::{convert::TryFrom, fmt};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::{
    decode::Decode,
    encode::{Encode, IsNull},
    error::BoxDynError,
    postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
    FromRow, Postgres, Type, TypeInfo,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl Role {
    fn parse_normalized(value: &str) -> Result<Self, RoleParseError> {
        match value {
            "employee" => Ok(Role::Employee),
            "manager" => Ok(Role::Manager),
            "finance" => Ok(Role::Finance),
            "admin" => Ok(Role::Admin),
            _ => Err(RoleParseError::new(value)),
        }
    }
}

impl TryFrom<&str> for Role {
    type Error = RoleParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        Role::parse_normalized(&normalized)
    }
}

impl Type<Postgres> for Role {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("employee_role")
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        matches!(ty.name(), "employee_role" | "text" | "varchar" | "bpchar")
    }
}

impl PgHasArrayType for Role {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_employee_role")
    }
}

impl<'q> Encode<'q, Postgres> for Role {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let value = self.as_str();
        <&str as Encode<Postgres>>::encode_by_ref(&value, buf)
    }

    fn size_hint(&self) -> usize {
        let value = self.as_str();
        <&str as Encode<Postgres>>::size_hint(&value)
    }
}

impl<'r> Decode<'r, Postgres> for Role {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let raw = <&str as Decode<Postgres>>::decode(value)?;
        Role::try_from(raw).map_err(|err| Box::new(err) as BoxDynError)
    }
}

#[derive(Debug, Clone)]
pub struct RoleParseError {
    value: String,
}

impl RoleParseError {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }
}

impl fmt::Display for RoleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported role value: {}", self.value)
    }
}

impl std::error::Error for RoleParseError {}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Employee {
    pub id: Uuid,
    pub hr_identifier: String,
    pub manager_id: Option<Uuid>,
    pub department: Option<String>,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[sqlx(type_name = "report_status", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "expense_category", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Type)]
#[sqlx(type_name = "approval_status", rename_all = "snake_case")]
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
