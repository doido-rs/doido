use crate::user_model::plan::FetchPlan;

pub fn select_by_email(plan: &FetchPlan) -> String {
    format!("SELECT {} FROM users WHERE email = ?", plan.column_list())
}

pub fn select_by_id(plan: &FetchPlan) -> String {
    format!("SELECT {} FROM users WHERE id = ?", plan.column_list())
}

pub fn select_by_column(plan: &FetchPlan, column: &str) -> String {
    format!(
        "SELECT {} FROM users WHERE {column} = ?",
        plan.column_list()
    )
}
