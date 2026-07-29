//! Authenticated encryption (AES-256-GCM) for cookies and credentials.
//!
//! [`encrypt`]/[`decrypt`] take an arbitrary-length secret (e.g. an app's
//! `secret_key_base` or `master.key`), derive a 32-byte key from it with
//! SHA-256, and produce/consume a `nonce(12) || ciphertext+tag` blob. GCM's
//! authentication tag means a tampered blob fails to decrypt (returns `None`),
//! so this both hides and authenticates the payload.
//!
//! Shared by the session cookie store (`doido-controller`) and encrypted
//! credentials (`doido-generators`).

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use sha2::{Digest, Sha256};

/// Nonce length for AES-GCM (96 bits, the standard).
const NONCE_LEN: usize = 12;

/// Derive a 32-byte AES-256 key from an arbitrary-length secret via SHA-256.
pub fn key_from_secret(secret: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.finalize().into()
}

/// Generate a random 32-byte secret, hex-encoded (64 chars) — e.g. an app's
/// `master.key` for encrypted credentials.
pub fn generate_key_hex() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Encrypt `plaintext` under `secret`, returning `nonce || ciphertext+tag`.
pub fn encrypt(secret: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let key = key_from_secret(secret);
    let cipher = Aes256Gcm::new((&key).into());
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(&Nonce::from(nonce_bytes), plaintext)
        .expect("AES-256-GCM encryption never fails for valid keys/nonces");
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

/// Decrypt a `nonce || ciphertext+tag` blob produced by [`encrypt`]. Returns
/// `None` when the blob is truncated, the secret is wrong, or the tag fails to
/// authenticate (tampering).
pub fn decrypt(secret: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < NONCE_LEN {
        return None;
    }
    let key = key_from_secret(secret);
    let cipher = Aes256Gcm::new((&key).into());
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce_arr: [u8; NONCE_LEN] = nonce_bytes.try_into().ok()?;
    cipher.decrypt(&Nonce::from(nonce_arr), ciphertext).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_plaintext() {
        let secret = b"a-secret-key-base";
        let blob = encrypt(secret, b"hello world");
        assert_eq!(decrypt(secret, &blob).as_deref(), Some(&b"hello world"[..]));
    }

    #[test]
    fn wrong_secret_fails_to_decrypt() {
        let blob = encrypt(b"right", b"payload");
        assert!(decrypt(b"wrong", &blob).is_none());
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let secret = b"s";
        let mut blob = encrypt(secret, b"payload");
        let last = blob.len() - 1;
        blob[last] ^= 0xff;
        assert!(decrypt(secret, &blob).is_none());
    }

    #[test]
    fn distinct_nonces_produce_distinct_ciphertexts() {
        let secret = b"s";
        assert_ne!(encrypt(secret, b"same"), encrypt(secret, b"same"));
    }
}
