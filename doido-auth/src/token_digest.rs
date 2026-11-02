//! One-way digests for high-entropy tokens (password reset).

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// SHA-256 hex digest of `token` for storage (not for user passwords).
pub fn hash_opaque_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

/// Constant-time compare of `token` against a stored digest or legacy plaintext token.
pub fn token_matches_stored(stored: &str, token: &str) -> bool {
    let hashed = hash_opaque_token(token);
    if stored.as_bytes().ct_eq(hashed.as_bytes()).into() {
        return true;
    }
    // Legacy rows stored the raw token (UUID).
    stored.as_bytes().ct_eq(token.as_bytes()).into()
}
