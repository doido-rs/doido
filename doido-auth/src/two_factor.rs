//! Optional TOTP two-factor authentication (feature `auth-2fa`).

use crate::error::AuthError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

/// Enrollment payload returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorEnrollment {
    pub secret: String,
    pub otpauth_uri: String,
}

/// Generate a new TOTP secret and provisioning URI for enrollment.
pub fn enroll(email: &str, issuer: &str) -> Result<TwoFactorEnrollment, AuthError> {
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

fn rand_secret() -> [u8; 20] {
    use uuid::Uuid;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut out = [0u8; 20];
    out[..16].copy_from_slice(a.as_bytes());
    out[16..].copy_from_slice(&b.as_bytes()[..4]);
    out
}
