//! Persist password digest updates (rehash-on-login, recoverable reset).

use crate::error::AuthError;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

fn internal(e: impl std::fmt::Display) -> AuthError {
    AuthError::Internal(e.to_string())
}

type DigestSyncHook = fn(&str, &str);

static DIGEST_SYNC_HOOK: std::sync::RwLock<Option<DigestSyncHook>> = std::sync::RwLock::new(None);

/// Test helper: mirror digest updates into the in-memory auth test store.
pub fn set_digest_sync_hook(hook: DigestSyncHook) {
    *DIGEST_SYNC_HOOK.write().expect("digest hook lock") = Some(hook);
}

/// Replace `password_digest` for the user with `email`.
pub async fn update_password_digest_by_email(
    db: &DatabaseConnection,
    email: &str,
    digest: &str,
) -> Result<(), AuthError> {
    if let Some(hook) = *DIGEST_SYNC_HOOK.read().expect("digest hook lock") {
        hook(email, digest);
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE users SET password_digest = ? WHERE email = ?",
        [
            Value::from(digest.to_string()),
            Value::from(email.to_string()),
        ],
    ))
    .await
    .map_err(internal)?;
    Ok(())
}
