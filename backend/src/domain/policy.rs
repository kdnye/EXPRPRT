use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::domain::models::{ExpenseCategory, ExpenseItem, PolicyCap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub is_valid: bool,
    pub violations: Vec<String>,
}

impl PolicyEvaluation {
    pub fn ok() -> Self {
        Self {
            is_valid: true,
            violations: Vec::new(),
        }
    }

    pub fn with_violation(message: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            violations: vec![message.into()],
        }
    }
}

pub fn evaluate_item(item: &ExpenseItem, caps: &[PolicyCap]) -> PolicyEvaluation {
    match item.category {
        ExpenseCategory::Meal => check_meal(item, caps),
        ExpenseCategory::Mileage => check_mileage(item, caps),
        _ => PolicyEvaluation::ok(),
    }
}

fn check_meal(item: &ExpenseItem, caps: &[PolicyCap]) -> PolicyEvaluation {
    let mut violations = Vec::new();
    for cap in caps.iter().filter(|c| c.category == ExpenseCategory::Meal) {
        if !cap_active(cap, item.expense_date) {
            continue;
        }
        if item.amount_cents > cap.amount_cents {
            violations.push(format!(
                "Meal exceeds per-diem limit of ${:.2}",
                cap.amount_cents as f64 / 100.0
            ));
        }
    }
    if violations.is_empty() {
        PolicyEvaluation::ok()
    } else {
        PolicyEvaluation {
            is_valid: false,
            violations,
        }
    }
}

fn check_mileage(item: &ExpenseItem, caps: &[PolicyCap]) -> PolicyEvaluation {
    let Some(cap) = caps
        .iter()
        .find(|c| c.category == ExpenseCategory::Mileage && cap_active(c, item.expense_date))
    else {
        return PolicyEvaluation::ok();
    };
    // For mileage the amount_cents represents the reimbursement amount already computed.
    if item.amount_cents <= cap.amount_cents {
        PolicyEvaluation::ok()
    } else {
        PolicyEvaluation::with_violation("Mileage exceeds configured reimbursement rate")
    }
}

fn cap_active(cap: &PolicyCap, date: NaiveDate) -> bool {
    let after_start = date >= cap.active_from;
    let before_end = cap.active_to.map(|d| date <= d).unwrap_or(true);
    after_start && before_end
}

pub fn current_fiscal_year(date: NaiveDate) -> (i32, i32) {
    let year = date.year();
    if date.month() >= 10 {
        (year, year + 1)
    } else {
        (year - 1, year)
    }
}
