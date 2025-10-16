use crate::domain::{
    models::ExpenseItem,
    policy::{evaluate_item, PolicyEvaluation},
};

pub fn validate_item(
    item: &ExpenseItem,
    caps: &[crate::domain::models::PolicyCap],
) -> PolicyEvaluation {
    evaluate_item(item, caps)
}
