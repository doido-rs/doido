//! `validatable` module — email format + password length checks on registration
//! (the Devise `validatable` analogue). Gated at runtime by `auth.modules`.

use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use doido_model::password::password_too_long;

/// Validate an email/password pair for registration when the `validatable`
/// module is enabled. A no-op when auth state isn't initialised (unit tests) or
/// the module is disabled, so callers can invoke it unconditionally.
pub fn validate_registration(email: &str, password: &str) -> Result<(), AuthError> {
    let state = match try_global() {
        Some(state) => state,
        None => return Ok(()),
    };
    if !state.config.has_module(AuthModule::Validatable) {
        return Ok(());
    }
    validate_email(email)?;
    validate_password_policy(password, state.config.effective_min_password_length())?;
    Ok(())
}

/// Password policy for reset / change-password flows (respects `validatable` gating).
pub fn validate_password_change(password: &str) -> Result<(), AuthError> {
    let state = match try_global() {
        Some(state) => state,
        None => return Ok(()),
    };
    if !state.config.has_module(AuthModule::Validatable) {
        return Ok(());
    }
    validate_password_policy(password, state.config.effective_min_password_length())
}

fn validate_password_policy(password: &str, min: usize) -> Result<(), AuthError> {
    validate_password_length(password, min)?;
    if password_too_long(password) {
        return Err(AuthError::Validation(
            "password is too long (maximum length exceeded)".into(),
        ));
    }
    Ok(())
}

/// A conservative email format check (`local@domain`, a dot in the domain, no
/// whitespace). Returns a [`AuthError::Validation`] on failure.
pub fn validate_email(email: &str) -> Result<(), AuthError> {
    if is_valid_email(email) {
        Ok(())
    } else {
        Err(AuthError::Validation("email is invalid".into()))
    }
}

/// Enforce a minimum password length.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_reasonable_emails() {
        for e in ["a@b.com", "user.name@example.co", "x@sub.domain.io"] {
            assert!(validate_email(e).is_ok(), "{e} should be valid");
        }
    }

    #[test]
    fn rejects_bad_emails() {
        for e in ["", "no-at", "a@b", "@b.com", "a b@c.com", "a@.com", "a@b."] {
            assert!(validate_email(e).is_err(), "{e} should be invalid");
        }
    }

    #[test]
    fn enforces_minimum_password_length() {
        assert!(validate_password_length("secretpassword!", 15).is_ok());
        assert!(validate_password_length("short", 15).is_err());
    }
}
