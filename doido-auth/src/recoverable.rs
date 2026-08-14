//! `recoverable` module — password reset via an emailed token (the Devise
//! `recoverable` analogue). Operates on the conventional `users` columns
//! (`reset_password_token`, `reset_password_sent_at`) via backend-agnostic SQL,
//! gated at runtime by `auth.modules`. Delivery uses the global `doido-mailer`
//! deliverer, so tests can capture the mail with a `TestDeliverer`.

use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use doido_mailer::Mail;
use doido_model::password::hash_password;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

fn enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Recoverable))
}

fn internal(e: impl std::fmt::Display) -> AuthError {
    AuthError::Internal(e.to_string())
}

/// Generate and store a reset token for `email` when the user exists. Returns the
/// token to embed in the reset email, or `None` (user unknown or module disabled).
/// Callers should always respond generically to avoid leaking account existence.
pub async fn request_reset(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<String>, AuthError> {
    if !enabled() {
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
        "UPDATE users SET reset_password_token = ?, reset_password_sent_at = ? WHERE email = ?",
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

/// Reset the password for the user holding `token`, when the token is valid and
/// unexpired (`auth.reset_password_within`). Hashes `new_password`, updates the
/// digest, and clears the token. Returns `true` on success; `false` on an
/// unknown/expired token or when the module is disabled.
pub async fn reset_password(
    db: &DatabaseConnection,
    token: &str,
    new_password: &str,
) -> Result<bool, AuthError> {
    let within = match try_global() {
        Some(state) if state.config.has_module(AuthModule::Recoverable) => {
            state.config.reset_password_within
        }
        _ => return Ok(false),
    };

    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT reset_password_sent_at FROM users WHERE reset_password_token = ?",
            [Value::from(token.to_string())],
        ))
        .await
        .map_err(internal)?;
    let sent_at = match row {
        Some(row) => row
            .try_get::<Option<String>>("", "reset_password_sent_at")
            .map_err(internal)?,
        None => return Ok(false),
    };
    if let Some(sent_at) = sent_at {
        let sent = chrono::DateTime::parse_from_rfc3339(&sent_at)
            .map(|t| t.with_timezone(&chrono::Utc))
            .map_err(internal)?;
        if (chrono::Utc::now() - sent).num_seconds() > within {
            return Ok(false);
        }
    }

    let digest = hash_password(new_password).map_err(internal)?;
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET password_digest = ?, reset_password_token = NULL, \
         reset_password_sent_at = NULL WHERE reset_password_token = ?",
        [Value::from(digest), Value::from(token.to_string())],
    ))
    .await
    .map_err(internal)?;
    Ok(true)
}

/// Deliver the reset-instructions email for `token` via the global deliverer.
/// Best-effort — the caller should not fail the request if delivery errors.
pub async fn send_reset_email(email: &str, token: &str) -> Result<(), AuthError> {
    let prefix = match try_global() {
        Some(state) => state.config.routes.prefix.trim_end_matches('/').to_string(),
        None => "/users".to_string(),
    };
    let url = format!("{prefix}/password/edit?reset_password_token={token}");
    let mail = Mail::new()
        .to(email)
        .subject("Reset your password")
        .body_text(format!(
            "Someone requested a password reset. Use this link to choose a new password:\n\n{url}\n\nIf you didn't request this, ignore this email."
        ));
    doido_mailer::global::deliverer()
        .deliver(&mail)
        .await
        .map_err(internal)
}
