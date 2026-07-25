//! Secure passwords and tokens (Rails `has_secure_password` / `has_secure_token`).
//!
//! Passwords are hashed with bcrypt; a model exposes its stored digest via
//! [`HasSecurePassword`] to gain [`authenticate`](HasSecurePassword::authenticate).

use bcrypt::{hash, verify, DEFAULT_COST};
use doido_core::Result;

/// Hash a password with bcrypt at the default cost.
pub fn hash_password(password: &str) -> Result<String> {
    hash_password_with_cost(password, DEFAULT_COST)
}

/// Hash a password with an explicit bcrypt cost (higher = slower/safer).
pub fn hash_password_with_cost(password: &str, cost: u32) -> Result<String> {
    hash(password, cost).map_err(|e| doido_core::anyhow::anyhow!("bcrypt hash failed: {e}"))
}

/// Verify a plaintext password against a bcrypt digest (false on any error).
pub fn verify_password(password: &str, digest: &str) -> bool {
    verify(password, digest).unwrap_or(false)
}

/// Generate an unguessable token (256 bits, hex) for `has_secure_token` and
/// one-off tokens (password reset, etc.).
pub fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Gives a model that stores a bcrypt digest an `authenticate` method.
pub trait HasSecurePassword {
    /// The stored bcrypt digest (e.g. the `password_digest` column).
    fn password_digest(&self) -> &str;

    /// Return `true` when `password` matches the stored digest.
    fn authenticate(&self, password: &str) -> bool {
        verify_password(password, self.password_digest())
    }
}
