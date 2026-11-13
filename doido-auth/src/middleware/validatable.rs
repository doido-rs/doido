//! Email/password validation on registration.

use super::context::SignUpContext;
use super::context::ValidateContext;
use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use async_trait::async_trait;

pub struct ValidatableMiddleware;

fn enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Validatable))
}

pub fn validate_registration(email: &str, password: &str) -> Result<(), AuthError> {
    let state = match try_global() {
        Some(state) => state,
        None => return Ok(()),
    };
    if !state.config.has_module(AuthModule::Validatable) {
        return Ok(());
    }
    validate_email(email)?;
    validate_password_length(password, state.config.password_length)?;
    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), AuthError> {
    if is_valid_email(email) {
        Ok(())
    } else {
        Err(AuthError::Validation("email is invalid".into()))
    }
}

pub fn validate_password_length(password: &str, min: usize) -> Result<(), AuthError> {
    if password.chars().count() >= min {
        Ok(())
    } else {
        Err(AuthError::Validation(format!(
            "password is too short (minimum is {min} characters)"
        )))
    }
}

fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.chars().any(char::is_whitespace) {
        return false;
    }
    match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        None => false,
    }
}

#[async_trait]
impl AuthMiddleware for ValidatableMiddleware {
    fn name(&self) -> &'static str {
        "validatable"
    }

    async fn before_validate(&self, ctx: &ValidateContext<'_>) -> Result<(), AuthError> {
        if enabled() {
            validate_registration(ctx.email, ctx.password)?;
        }
        Ok(())
    }

    async fn before_sign_up(&self, ctx: &SignUpContext<'_>) -> Result<(), AuthError> {
        if enabled() {
            validate_registration(ctx.email, ctx.password)?;
        }
        Ok(())
    }
}
