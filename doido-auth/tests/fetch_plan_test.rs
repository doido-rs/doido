//! Fetch plan composition for sign-in.

use doido_auth::config::{AuthConfig, AuthModule};
use doido_auth::user_model::FetchPlan;

#[test]
fn sign_in_plan_includes_lockable_and_confirmable_columns() {
    let config = AuthConfig {
        modules: vec![
            AuthModule::DatabaseAuthenticatable,
            AuthModule::Lockable,
            AuthModule::Confirmable,
        ],
        ..Default::default()
    };
    let plan = FetchPlan::for_sign_in(&config);
    let cols = plan.column_list();
    assert!(cols.contains("failed_attempts"));
    assert!(cols.contains("locked_at"));
    assert!(cols.contains("confirmed_at"));
    assert!(cols.contains("email"));
}
