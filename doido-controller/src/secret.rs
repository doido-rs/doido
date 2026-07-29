//! The process-global secret key base used to sign/encrypt cookies (session,
//! flash, signed cookie jar).
//!
//! An app installs its real secret once at boot with [`set_key_base`] (from
//! config/credentials); until then a fixed **dev-insecure** default is used so
//! things work out of the box in development. Never ship the default.

use std::sync::OnceLock;

static SECRET: OnceLock<Vec<u8>> = OnceLock::new();

/// The insecure development default. Real apps must override it at boot.
const DEV_SECRET: &[u8] = b"doido-dev-insecure-secret-key-base-change-me";

/// Install the app's secret key base. Idempotent: a second call is rejected and
/// returns the passed-in secret back. Call once at boot before serving.
pub fn set_key_base(secret: impl Into<Vec<u8>>) -> Result<(), Vec<u8>> {
    SECRET.set(secret.into())
}

/// The configured secret key base, or the dev-insecure default when unset.
pub fn key_base() -> Vec<u8> {
    SECRET.get().cloned().unwrap_or_else(|| DEV_SECRET.to_vec())
}
