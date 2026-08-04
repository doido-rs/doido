//! Two-factor tests (feature `auth-2fa`).

#![cfg(feature = "auth-2fa")]

use doido_auth::two_factor::{enroll, verify_code};

#[test]
fn enroll_produces_secret_and_uri() {
    let enrollment = enroll("user@example.com", "MyApp").unwrap();
    assert!(enrollment.otpauth_uri.contains("otpauth://totp/MyApp"));
    assert!(!enrollment.secret.is_empty());
    assert!(!verify_code(&enrollment.secret, "000000").unwrap());
}
