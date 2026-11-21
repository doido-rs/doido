use crate::user_model::plan::FetchPlan;

pub fn select_by_email(plan: &FetchPlan) -> String {
    format!("SELECT {} FROM users WHERE email = $1", plan.column_list())
}

pub fn select_by_id(plan: &FetchPlan) -> String {
    format!("SELECT {} FROM users WHERE id = $1", plan.column_list())
}

pub fn select_by_column(plan: &FetchPlan, column: &str) -> String {
    format!(
        "SELECT {} FROM users WHERE {column} = $1",
        plan.column_list()
    )
}
