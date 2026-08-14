//! `trackable` module — records sign-in statistics (count, timestamps, IPs) on
//! the conventional `users` columns. Gated at runtime by `auth.modules`.

use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

/// Returns whether the `trackable` module is enabled in the current auth state.
fn enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Trackable))
}

/// Record a successful sign-in for the user identified by `email`: advance
/// `sign_in_count`, roll `current_*` → `last_*` timestamp/IP, and stamp the new
/// current sign-in. No-op when the `trackable` module is disabled (or auth state
/// isn't initialised), so callers can invoke it unconditionally.
pub async fn record_sign_in(
    db: &DatabaseConnection,
    email: &str,
    ip: Option<&str>,
) -> Result<(), AuthError> {
    if !enabled() {
        return Ok(());
    }
    let now = chrono::Utc::now().to_rfc3339();
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET \
            sign_in_count = sign_in_count + 1, \
            last_sign_in_at = current_sign_in_at, \
            current_sign_in_at = ?, \
            last_sign_in_ip = current_sign_in_ip, \
            current_sign_in_ip = ? \
         WHERE email = ?",
        [
            Value::from(now),
            Value::from(ip.map(|s| s.to_string())),
            Value::from(email.to_string()),
        ],
    );
    db.execute_raw(stmt)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(())
}
