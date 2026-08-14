//! `confirmable` module — require email confirmation before sign-in (the Devise
//! `confirmable` analogue). Operates on the conventional `users` columns
//! (`confirmation_token`, `confirmed_at`, `confirmation_sent_at`) via
//! backend-agnostic SQL, gated at runtime by `auth.modules`. Delivery uses the
//! global `doido-mailer` deliverer.

use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use doido_mailer::Mail;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

/// Whether the `confirmable` module is enabled in the current auth state.
pub fn is_enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Confirmable))
}

fn internal(e: impl std::fmt::Display) -> AuthError {
    AuthError::Internal(e.to_string())
}

/// Whether `email`'s account has confirmed its address. Returns `true` when the
/// module is disabled (no gating) and `false` for an unknown email.
pub async fn is_confirmed(db: &DatabaseConnection, email: &str) -> Result<bool, AuthError> {
    if !is_enabled() {
        return Ok(true);
    }
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT confirmed_at FROM users WHERE email = ?",
            [Value::from(email.to_string())],
        ))
        .await
        .map_err(internal)?;
    match row {
        Some(row) => Ok(row
            .try_get::<Option<String>>("", "confirmed_at")
            .map_err(internal)?
            .is_some()),
        None => Ok(false),
    }
}

/// Generate and store a confirmation token for `email` (if the user exists).
/// Returns the token to embed in the confirmation email, or `None`.
pub async fn generate_confirmation(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<String>, AuthError> {
    if !is_enabled() {
        return Ok(None);
    }
    let exists = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM users WHERE email = ?",
            [Value::from(email.to_string())],
        ))
        .await
        .map_err(internal)?
        .is_some();
    if !exists {
        return Ok(None);
    }
    let token = uuid::Uuid::new_v4().to_string();
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET confirmation_token = ?, confirmation_sent_at = ? WHERE email = ?",
        [
            Value::from(token.clone()),
            Value::from(chrono::Utc::now().to_rfc3339()),
            Value::from(email.to_string()),
        ],
    ))
    .await
    .map_err(internal)?;
    Ok(Some(token))
}

/// Confirm the account holding `token`: stamp `confirmed_at` and clear the token.
/// Returns `true` on success; `false` for an unknown token or disabled module.
pub async fn confirm(db: &DatabaseConnection, token: &str) -> Result<bool, AuthError> {
    if !is_enabled() {
        return Ok(false);
    }
    let exists = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM users WHERE confirmation_token = ?",
            [Value::from(token.to_string())],
        ))
        .await
        .map_err(internal)?
        .is_some();
    if !exists {
        return Ok(false);
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET confirmed_at = ?, confirmation_token = NULL WHERE confirmation_token = ?",
        [
            Value::from(chrono::Utc::now().to_rfc3339()),
            Value::from(token.to_string()),
        ],
    ))
    .await
    .map_err(internal)?;
    Ok(true)
}

/// Deliver the confirmation-instructions email for `token` via the global
/// deliverer. Best-effort — callers should not fail the request on delivery error.
pub async fn send_confirmation_email(email: &str, token: &str) -> Result<(), AuthError> {
    let prefix = match try_global() {
        Some(state) => state.config.routes.prefix.trim_end_matches('/').to_string(),
        None => "/users".to_string(),
    };
    let url = format!("{prefix}/confirmation?confirmation_token={token}");
    let mail = Mail::new()
        .to(email)
        .subject("Confirm your email")
        .body_text(format!(
            "Welcome! Confirm your email address to activate your account:\n\n{url}\n"
        ));
    doido_mailer::global::deliverer()
        .deliver(&mail)
        .await
        .map_err(internal)
}
