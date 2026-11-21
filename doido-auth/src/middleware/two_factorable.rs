//! TOTP two-factor (`two_factor_authenticatable` module; middleware name `two_factorable`).

use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use crate::user_model::plan::{FetchContribution, UserField};
use crate::user_model::row::UserRow;
use crate::user_model::{UserPatchBuilder, UserStore};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

pub struct TwoFactorableMiddleware;

/// Enrollment payload returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorEnrollment {
    pub secret: String,
    pub otpauth_uri: String,
}

/// App `UserModel` types that expose 2FA columns.
pub trait HasTwoFactor {
    fn two_factor_secret(&self) -> Option<&str>;
    fn two_factor_enabled(&self) -> bool;
}

/// Controller helper (ejected templates call [`TwoFactor::verify`]).
pub struct TwoFactor;

impl TwoFactor {
    pub fn verify<U: HasTwoFactor>(user: &U, code: &str) -> Result<(), AuthError> {
        if !user.two_factor_enabled() {
            return Ok(());
        }
        let secret = user.two_factor_secret().ok_or_else(|| {
            AuthError::TwoFactor("two-factor is enabled but secret is missing".into())
        })?;
        if verify_code(secret, code)? {
            Ok(())
        } else {
            Err(AuthError::TwoFactor("invalid verification code".into()))
        }
    }
}

pub fn is_module_enabled() -> bool {
    matches!(
        try_global(),
        Some(state) if state.config.has_module(AuthModule::TwoFactorAuthenticatable)
    )
}

pub fn two_factor_enabled(row: &UserRow) -> bool {
    row.two_factor_enabled.unwrap_or(false)
}

pub fn requires_challenge(row: &UserRow) -> bool {
    is_module_enabled() && two_factor_enabled(row) && row.two_factor_secret.is_some()
}

pub fn validate_row(row: &UserRow) -> Result<(), AuthError> {
    if !is_module_enabled() {
        return Ok(());
    }
    if requires_challenge(row) {
        return Err(AuthError::TwoFactor(
            "two-factor challenge required before sign-in completes".into(),
        ));
    }
    Ok(())
}

/// Generate a new TOTP secret and provisioning URI for enrollment.
pub fn enroll(email: &str, issuer: &str) -> Result<TwoFactorEnrollment, AuthError> {
    let issuer = issuer_from_config().unwrap_or_else(|| issuer.to_string());
    let secret_bytes: [u8; 20] = rand_secret();
    let secret = STANDARD.encode(secret_bytes);
    let otpauth_uri = format!(
        "otpauth://totp/{issuer}:{email}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits=6&period=30"
    );
    Ok(TwoFactorEnrollment {
        secret: secret.clone(),
        otpauth_uri,
    })
}

/// Verify a 6-digit TOTP code against a base64-encoded secret.
pub fn verify_code(secret_b64: &str, code: &str) -> Result<bool, AuthError> {
    let secret = STANDARD
        .decode(secret_b64)
        .map_err(|e| AuthError::TwoFactor(format!("invalid secret: {e}")))?;
    let expected = totp_lite::totp_custom::<totp_lite::Sha1>(30, 6, &secret, 0);
    Ok(expected == code)
}

pub async fn enable_for_user(
    store: &dyn UserStore,
    email: &str,
    secret_b64: &str,
) -> Result<(), AuthError> {
    if !is_module_enabled() {
        return Ok(());
    }
    let action = UserPatchBuilder::by_email(email)
        .set(UserField::TwoFactorSecret, secret_b64)
        .set(UserField::TwoFactorEnabled, "true")
        .build()?;
    store.apply(action).await
}

pub async fn disable_for_user(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    if !is_module_enabled() {
        return Ok(());
    }
    let action = UserPatchBuilder::by_email(email)
        .clear(UserField::TwoFactorSecret)
        .set(UserField::TwoFactorEnabled, "false")
        .build()?;
    store.apply(action).await
}

pub async fn verify_user_code(
    store: &dyn UserStore,
    email: &str,
    code: &str,
) -> Result<bool, AuthError> {
    let plan = crate::user_model::FetchPlan::minimal().merge(
        FetchContribution::default()
            .with(UserField::TwoFactorSecret)
            .with(UserField::TwoFactorEnabled),
    );
    let row = store.fetch_by_email(email, &plan).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    if !two_factor_enabled(&row) {
        return Ok(true);
    }
    let secret = match row.two_factor_secret.as_deref() {
        Some(s) => s,
        None => return Ok(false),
    };
    verify_code(secret, code)
}

fn issuer_from_config() -> Option<String> {
    try_global().and_then(|s| s.config.two_factor.issuer.clone())
}

fn rand_secret() -> [u8; 20] {
    use uuid::Uuid;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut out = [0u8; 20];
    out[..16].copy_from_slice(a.as_bytes());
    out[16..].copy_from_slice(&b.as_bytes()[..4]);
    out
}

#[async_trait]
impl AuthMiddleware for TwoFactorableMiddleware {
    fn name(&self) -> &'static str {
        "two_factorable"
    }

    fn sign_in_fetch_plan(&self) -> FetchContribution {
        FetchContribution::default()
            .with(UserField::TwoFactorSecret)
            .with(UserField::TwoFactorEnabled)
    }
}
