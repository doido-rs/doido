use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use doido_core::{anyhow::Context as _, Result};
use std::path::Path;

/// Encrypts `plaintext` with `key` using AES-256-GCM with a random nonce.
/// Returns a base64-encoded blob: `nonce(12 bytes) || ciphertext`.
pub fn encrypt_credentials(plaintext: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| doido_core::anyhow::anyhow!("AES-GCM encryption failed"))?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(&out))
}

/// Decrypts a base64-encoded blob produced by `encrypt_credentials`.
pub fn decrypt_credentials(encoded: &str, key: &[u8; 32]) -> Result<String> {
    let raw = STANDARD
        .decode(encoded.trim())
        .map_err(|e| doido_core::anyhow::anyhow!("base64 decode failed: {e}"))?;
    if raw.len() < 12 {
        doido_core::anyhow::bail!("credentials blob too short to contain nonce");
    }
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| doido_core::anyhow::anyhow!("decryption failed — wrong key?"))?;
    String::from_utf8(plaintext)
        .map_err(|e| doido_core::anyhow::anyhow!("credentials are not valid UTF-8: {e}"))
}

/// Resolves the 32-byte master key:
/// 1. `DOIDO_MASTER_KEY` env var (64-char hex string)
/// 2. `config/master.key` file (64-char hex string, trailing whitespace trimmed)
pub fn load_master_key(root: &Path) -> Result<[u8; 32]> {
    let hex_str = std::env::var("DOIDO_MASTER_KEY").or_else(|_| {
        let key_path = root.join("config/master.key");
        std::fs::read_to_string(&key_path)
            .map(|s| s.trim().to_string())
            .map_err(|e| doido_core::anyhow::anyhow!("cannot read config/master.key: {e}"))
    })?;
    let bytes = hex::decode(hex_str.trim()).context("master key is not valid hex")?;
    bytes
        .try_into()
        .map_err(|_| doido_core::anyhow::anyhow!("master key must be 32 bytes (64 hex chars)"))
}
