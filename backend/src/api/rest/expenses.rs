use std::{collections::BTreeMap, sync::Arc};

use axum::http::StatusCode;
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    domain::models::ExpenseCategory,
    infrastructure::{auth::AuthenticatedUser, state::AppState},
    services::errors::ServiceError,
    services::expenses::{
        CreateExpenseItem, CreateReceiptReference, CreateReportRequest, ExpenseService,
    },
};

use crate::infrastructure::config::ReceiptRules;

#[derive(Debug, serde::Deserialize)]
struct CreateReportPayload {
    reporting_period_start: chrono::NaiveDate,
    reporting_period_end: chrono::NaiveDate,
    currency: String,
    #[serde(default)]
    items: Vec<CreateReportItemPayload>,
}

#[derive(Debug, serde::Deserialize)]
struct CreateReportItemPayload {
    expense_date: chrono::NaiveDate,
    category: ExpenseCategory,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    attendees: Option<String>,
    #[serde(default)]
    location: Option<String>,
    amount_cents: i64,
    reimbursable: bool,
    #[serde(default)]
    payment_method: Option<String>,
    #[serde(default)]
    receipts: Vec<ReceiptPayload>,
}

#[derive(Debug, serde::Deserialize)]
struct ReceiptPayload {
    file_key: String,
    file_name: String,
    mime_type: String,
    size_bytes: i64,
}

pub fn router() -> Router {
    Router::new()
        .route("/reports", post(create_report))
        .route("/reports/:id/submit", post(submit_report))
        .route("/reports/:id/policy", get(evaluate_report))
}

async fn create_report(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateReportPayload>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let validation_errors = validate_create_report_payload(&payload, &state.config.receipts);
    if !validation_errors.is_empty() {
        return Err(validation_error_response(validation_errors));
    }

    let service = ExpenseService::new(state);
    let report = service
        .create_report(&user, payload.into_request())
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "report": report })))
}

async fn submit_report(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = ExpenseService::new(state);
    let report = service
        .submit_report(&user, id)
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "report": report })))
}

async fn evaluate_report(
    Extension(state): Extension<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let service = ExpenseService::new(state);
    let result = service
        .evaluate_report(&user, id)
        .await
        .map_err(to_response)?;
    Ok(Json(serde_json::json!({ "evaluation": result })))
}

fn to_response(err: ServiceError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    match err {
        ServiceError::Validation(message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": "validation_failed",
                "message": message,
            })),
        ),
        other => (
            other.status_code(),
            Json(serde_json::json!({ "error": other.to_string() })),
        ),
    }
}

impl CreateReportPayload {
    fn into_request(self) -> CreateReportRequest {
        CreateReportRequest {
            reporting_period_start: self.reporting_period_start,
            reporting_period_end: self.reporting_period_end,
            currency: self.currency,
            items: self
                .items
                .into_iter()
                .map(|item| CreateExpenseItem {
                    expense_date: item.expense_date,
                    category: item.category,
                    description: item.description,
                    attendees: item.attendees,
                    location: item.location,
                    amount_cents: item.amount_cents,
                    reimbursable: item.reimbursable,
                    payment_method: item.payment_method,
                    receipts: item
                        .receipts
                        .into_iter()
                        .map(|receipt| CreateReceiptReference {
                            file_key: receipt.file_key,
                            file_name: receipt.file_name,
                            mime_type: receipt.mime_type,
                            size_bytes: receipt.size_bytes,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

fn validate_create_report_payload(
    payload: &CreateReportPayload,
    receipt_rules: &ReceiptRules,
) -> BTreeMap<String, Vec<String>> {
    let mut errors: BTreeMap<String, Vec<String>> = BTreeMap::new();

    if payload.currency.trim().is_empty() {
        push_error(&mut errors, "currency", "currency is required");
    }

    if payload.reporting_period_end < payload.reporting_period_start {
        push_error(
            &mut errors,
            "reporting_period_end",
            "must be on or after reporting_period_start",
        );
    }

    if payload.items.is_empty() {
        push_error(
            &mut errors,
            "items",
            "at least one expense item is required",
        );
        return errors;
    }

    for (index, item) in payload.items.iter().enumerate() {
        if item.amount_cents <= 0 {
            push_error(
                &mut errors,
                format!("items.{index}.amount_cents"),
                "must be greater than 0",
            );
        }

        if item.expense_date < payload.reporting_period_start
            || item.expense_date > payload.reporting_period_end
        {
            push_error(
                &mut errors,
                format!("items.{index}.expense_date"),
                "must be within the reporting period",
            );
        }

        if item.receipts.len() as u32 > receipt_rules.max_files_per_item {
            push_error(
                &mut errors,
                format!("items.{index}.receipts"),
                format!(
                    "cannot attach more than {} receipts",
                    receipt_rules.max_files_per_item
                ),
            );
        }

        for (receipt_index, receipt) in item.receipts.iter().enumerate() {
            if receipt.file_key.trim().is_empty() {
                push_error(
                    &mut errors,
                    format!("items.{index}.receipts.{receipt_index}.file_key"),
                    "file_key is required",
                );
            }

            if receipt.file_name.trim().is_empty() {
                push_error(
                    &mut errors,
                    format!("items.{index}.receipts.{receipt_index}.file_name"),
                    "file_name is required",
                );
            }

            if receipt.mime_type.trim().is_empty() {
                push_error(
                    &mut errors,
                    format!("items.{index}.receipts.{receipt_index}.mime_type"),
                    "mime_type is required",
                );
            }

            if receipt.size_bytes <= 0 {
                push_error(
                    &mut errors,
                    format!("items.{index}.receipts.{receipt_index}.size_bytes"),
                    "must be greater than 0",
                );
            } else if receipt.size_bytes as u64 > receipt_rules.max_bytes {
                push_error(
                    &mut errors,
                    format!("items.{index}.receipts.{receipt_index}.size_bytes"),
                    format!("exceeds maximum size of {} bytes", receipt_rules.max_bytes),
                );
            }
        }
    }

    errors
}

fn push_error(
    errors: &mut BTreeMap<String, Vec<String>>,
    key: impl Into<String>,
    message: impl Into<String>,
) {
    errors.entry(key.into()).or_default().push(message.into());
}

fn validation_error_response(
    errors: BTreeMap<String, Vec<String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(serde_json::json!({ "errors": errors })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn maps_conflict_errors_to_http_409() {
        let (status, Json(body)) = to_response(ServiceError::Conflict);

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body, serde_json::json!({ "error": "conflict" }));
    }

    #[test]
    fn maps_not_found_errors_to_http_404() {
        let (status, Json(body)) = to_response(ServiceError::NotFound);

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, serde_json::json!({ "error": "not found" }));
    }

    #[test]
    fn maps_validation_errors_to_http_422() {
        let (status, Json(body)) = to_response(ServiceError::Validation("totals mismatch".into()));

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body,
            serde_json::json!({
                "error": "validation_failed",
                "message": "totals mismatch",
            })
        );
    }

    #[test]
    fn validate_create_report_payload_returns_structured_errors() {
        let payload = CreateReportPayload {
            reporting_period_start: chrono::NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
            reporting_period_end: chrono::NaiveDate::from_ymd_opt(2024, 5, 31).unwrap(),
            currency: "".to_string(),
            items: vec![CreateReportItemPayload {
                expense_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                category: ExpenseCategory::Meal,
                description: None,
                attendees: None,
                location: None,
                amount_cents: 0,
                reimbursable: true,
                payment_method: None,
                receipts: vec![ReceiptPayload {
                    file_key: "".to_string(),
                    file_name: "".to_string(),
                    mime_type: "".to_string(),
                    size_bytes: 0,
                }],
            }],
        };

        let errors = validate_create_report_payload(&payload, &ReceiptRules::default());

        assert_eq!(errors.get("currency").unwrap()[0], "currency is required");
        assert!(errors.contains_key("items.0.amount_cents"));
        assert!(errors.contains_key("items.0.expense_date"));
        assert!(errors.contains_key("items.0.receipts.0.file_key"));
        assert!(errors.contains_key("items.0.receipts.0.size_bytes"));
    }
}
