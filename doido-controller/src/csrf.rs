//! CSRF protection via the double-submit-cookie pattern.
//!
//! A random token is placed in a `csrf_token` cookie and echoed back by the
//! client in the `X-CSRF-Token` header (or a form field). The [`with_csrf`]
//! middleware (see [`crate::stack::MiddlewareStack`]) requires the two to match
//! on state-changing requests, so a cross-site request — which cannot read the
//! cookie to echo it — is rejected.

use subtle::ConstantTimeEq;

/// Mint a fresh, unguessable CSRF token (256 bits of randomness, hex-encoded).
pub fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Constant-time comparison of a stored token and a submitted one.
pub fn tokens_match(expected: &str, actual: &str) -> bool {
    expected.as_bytes().ct_eq(actual.as_bytes()).into()
}

/// Extract the `csrf_token` value from a `Cookie` header, if present.
pub(crate) fn token_from_cookie_header(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == "csrf_token")
        .map(|(_, value)| value.to_string())
}
