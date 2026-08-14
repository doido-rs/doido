//! Sign-in, sign-up, and sign-out helpers built on [`AuthUser`] and bcrypt.

use crate::error::AuthError;
use crate::session::{sign_in_session, sign_out_session};
use crate::user::{authenticate_password, AuthUser};
use doido_controller::session::Session;
use doido_controller::Context;
use doido_core::Result;
use doido_model::password::{hash_password, HasSecurePassword};
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
    let user = U::find_by_email(db, email)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
        .ok_or(AuthError::InvalidCredentials)?;

    if !authenticate_password(&user, password) {
        return Err(AuthError::InvalidCredentials);
    }

    Ok(user)
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
    // `validatable` module: email format + password length (no-op when disabled).
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
