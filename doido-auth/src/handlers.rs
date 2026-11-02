//! Sign-in, sign-up, and sign-out helpers built on [`AuthUser`] and password digests.

use crate::error::AuthError;
use crate::password_store::update_password_digest_by_email;
use crate::session::{sign_in_session, sign_out_session};
use crate::state::try_global;
use crate::user::{authenticate_password, AuthUser};
use doido_controller::session::Session;
use doido_controller::Context;
use doido_core::Result;
use doido_model::password::{hash_password, needs_rehash, HasSecurePassword};
use doido_model::sea_orm::DatabaseConnection;
use std::future::Future;

/// Authenticate by email/password and return the user.
pub async fn authenticate<U>(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
) -> Result<U, AuthError>
where
    U: AuthUser + HasSecurePassword,
{
    let user = match U::find_by_email(db, email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            if should_dummy_verify() {
                doido_model::password::dummy_password_verify(password);
            }
            return Err(AuthError::InvalidCredentials);
        }
        Err(e) => return Err(AuthError::Internal(e.to_string())),
    };

    // `lockable` module: reject locked accounts before checking the password.
    crate::lockable::ensure_not_locked(db, email).await?;

    if !authenticate_password(&user, password) {
        // `lockable`: count the failure (best-effort — never masks the auth error).
        let _ = crate::lockable::record_failure(db, email).await;
        return Err(AuthError::InvalidCredentials);
    }

    // Transparent Argon2id upgrade for legacy bcrypt (or outdated params).
    let digest = HasSecurePassword::password_digest(&user).to_string();
    let mut user = user;
    if needs_rehash(&digest) {
        if let Ok(new_digest) = hash_password(password) {
            let _ = update_password_digest_by_email(db, email, &new_digest).await;
            if let Ok(Some(fresh)) = U::find_by_email(db, email).await {
                user = fresh;
            }
        }
    }

    // `confirmable`: block sign-in until the email is confirmed.
    if !crate::confirmable::is_confirmed(db, email).await? {
        return Err(AuthError::NotConfirmed);
    }

    // `lockable`: clear the failed-attempt counter on success.
    let _ = crate::lockable::reset_attempts(db, email).await;
    Ok(user)
}

fn should_dummy_verify() -> bool {
    try_global()
        .map(|s| s.config.timing_attack_mitigation)
        .unwrap_or(true)
}

/// Persist `user_id` in the encrypted session cookie payload.
pub fn sign_in_with_session<U: AuthUser>(session: &mut Session, user: &U) {
    sign_in_session(session, user.id());
}

/// Establish an authenticated session for `user` on the request context.
pub fn sign_in(ctx: &mut Context, user: &impl AuthUser) -> Result<(), AuthError> {
    sign_in_with_session(ctx.session(), user);
    Ok(())
}

/// Clear the authenticated user from the session on the request context.
pub fn sign_out(ctx: &mut Context) -> Result<(), AuthError> {
    sign_out_session(ctx.session());
    Ok(())
}

/// Register a new user: checks email uniqueness, hashes the password, then calls `create`.
pub async fn register_user<U, F, Fut>(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
    create: F,
) -> Result<U, AuthError>
where
    U: AuthUser,
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = Result<U>>,
{
    crate::validations::validate_registration(email, password)?;

    if U::find_by_email(db, email)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(AuthError::EmailTaken);
    }

    let digest = hash_password(password).map_err(|e| AuthError::Internal(e.to_string()))?;
    create(email.to_string(), digest)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))
}
