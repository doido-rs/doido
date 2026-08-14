//! `lockable` module — locks an account after repeated failed sign-ins and
//! auto-unlocks after `auth.unlock_in` seconds. Operates on the conventional
//! `users` columns (`failed_attempts`, `locked_at`) via backend-agnostic SQL,
//! gated at runtime by `auth.modules`. Email-based unlock is a follow-up; the
//! time-based unlock strategy needs no mailer.

use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

struct Settings {
    maximum_attempts: u32,
    unlock_in: i64,
}

/// The lockable settings when the module is enabled, else `None`.
fn settings() -> Option<Settings> {
    let state = try_global()?;
    if !state.config.has_module(AuthModule::Lockable) {
        return None;
    }
    Some(Settings {
        maximum_attempts: state.config.maximum_attempts,
        unlock_in: state.config.unlock_in,
    })
}

async fn locked_at(db: &DatabaseConnection, email: &str) -> Result<Option<String>, AuthError> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT locked_at FROM users WHERE email = ?",
            [Value::from(email.to_string())],
        ))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    match row {
        Some(row) => Ok(row
            .try_get::<Option<String>>("", "locked_at")
            .map_err(|e| AuthError::Internal(e.to_string()))?),
        None => Ok(None),
    }
}

async fn exec(db: &DatabaseConnection, sql: &str, email: &str) -> Result<(), AuthError> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        [Value::from(email.to_string())],
    ))
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(())
}

/// Reject a sign-in attempt for a locked account. Auto-unlocks (and allows the
/// attempt) once `unlock_in` seconds have elapsed since `locked_at`. No-op when
/// the module is disabled.
pub async fn ensure_not_locked(db: &DatabaseConnection, email: &str) -> Result<(), AuthError> {
    let settings = match settings() {
        Some(s) => s,
        None => return Ok(()),
    };
    let locked = match locked_at(db, email).await? {
        Some(ts) => ts,
        None => return Ok(()),
    };
    let locked_time = chrono::DateTime::parse_from_rfc3339(&locked)
        .map(|t| t.with_timezone(&chrono::Utc))
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    if (chrono::Utc::now() - locked_time).num_seconds() >= settings.unlock_in {
        // Lock window elapsed — auto-unlock and let the attempt proceed.
        unlock(db, email).await?;
        Ok(())
    } else {
        Err(AuthError::AccountLocked)
    }
}

/// Record a failed sign-in: increment `failed_attempts` and lock the account
/// (stamp `locked_at`) once it reaches `maximum_attempts`. No-op when disabled.
pub async fn record_failure(db: &DatabaseConnection, email: &str) -> Result<(), AuthError> {
    let settings = match settings() {
        Some(s) => s,
        None => return Ok(()),
    };
    exec(
        db,
        "UPDATE users SET failed_attempts = failed_attempts + 1 WHERE email = ?",
        email,
    )
    .await?;

    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT failed_attempts FROM users WHERE email = ?",
            [Value::from(email.to_string())],
        ))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    let attempts = match row {
        Some(row) => row
            .try_get::<i32>("", "failed_attempts")
            .map_err(|e| AuthError::Internal(e.to_string()))?,
        None => return Ok(()),
    };

    if attempts as u32 >= settings.maximum_attempts {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE users SET locked_at = ? WHERE email = ?",
            [
                Value::from(chrono::Utc::now().to_rfc3339()),
                Value::from(email.to_string()),
            ],
        ))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    }
    Ok(())
}

/// Clear the failed-attempt counter and lock on a successful sign-in. No-op when
/// the module is disabled.
pub async fn reset_attempts(db: &DatabaseConnection, email: &str) -> Result<(), AuthError> {
    if settings().is_none() {
        return Ok(());
    }
    unlock(db, email).await
}

async fn unlock(db: &DatabaseConnection, email: &str) -> Result<(), AuthError> {
    exec(
        db,
        "UPDATE users SET failed_attempts = 0, locked_at = NULL WHERE email = ?",
        email,
    )
    .await
}
