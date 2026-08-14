//! `rememberable` module — a persistent "remember me" cookie that signs the user
//! back in across browser sessions (the Devise `rememberable` analogue). Stamps
//! `remember_created_at` and issues a signed, `Max-Age`-bearing cookie; a
//! [`RememberStrategy`] resolves it on later requests when no session is present.

use crate::config::AuthModule;
use crate::error::AuthError;
use crate::identity::AuthIdentity;
use crate::strategy::AuthStrategy;
use async_trait::async_trait;
use doido_controller::CookieJar;
use doido_core::Result;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};
use http::header;
use http::request::Parts;

/// Signed cookie holding the remembered user id.
pub const REMEMBER_COOKIE: &str = "_doido_remember";

fn enabled() -> bool {
    matches!(crate::state::try_global(), Some(state) if state.config.has_module(AuthModule::Rememberable))
}

/// The signed cookie value for a remembered user id (its JSON encoding, so it
/// round-trips back to `AuthUser::Id`).
pub fn cookie_value(user_id: &impl serde::Serialize) -> String {
    serde_json::to_string(user_id).unwrap_or_default()
}

/// Stamp `remember_created_at` for `email`. No-op when the module is disabled.
pub async fn record_remember(db: &DatabaseConnection, email: &str) -> Result<(), AuthError> {
    if !enabled() {
        return Ok(());
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET remember_created_at = ? WHERE email = ?",
        [
            Value::from(chrono::Utc::now().to_rfc3339()),
            Value::from(email.to_string()),
        ],
    ))
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(())
}

/// Clear `remember_created_at` for `email` (on sign-out). No-op when disabled.
pub async fn forget(db: &DatabaseConnection, email: &str) -> Result<(), AuthError> {
    if !enabled() {
        return Ok(());
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET remember_created_at = NULL WHERE email = ?",
        [Value::from(email.to_string())],
    ))
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(())
}

/// Auth strategy that resolves an identity from the signed remember cookie. Added
/// to the strategy chain automatically when the `rememberable` module is enabled,
/// so it only runs after the session/JWT strategies decline.
pub struct RememberStrategy;

#[async_trait]
impl AuthStrategy for RememberStrategy {
    fn name(&self) -> &str {
        "remember"
    }

    async fn authenticate(
        &self,
        parts: &Parts,
        _db: &DatabaseConnection,
    ) -> Result<Option<AuthIdentity>> {
        let header = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok());
        let jar = CookieJar::from_header(header, doido_controller::secret::key_base());
        match jar.get_signed(REMEMBER_COOKIE) {
            Some(raw) => {
                let user_id = serde_json::from_str::<serde_json::Value>(&raw)
                    .unwrap_or(serde_json::Value::String(raw));
                Ok(Some(AuthIdentity { user_id }))
            }
            None => Ok(None),
        }
    }
}
