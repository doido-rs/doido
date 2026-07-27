//! HMAC-SHA256 signed ids and signed URLs — the analogue of Rails' message
//! verifier used by ActiveStorage `signed_id`s and disk-service URLs.
//!
//! A token is `base64url(payload_json)--base64url(hmac)`, where `payload_json`
//! carries the signed data, a `purpose` string, and an optional expiry (unix
//! seconds). Verification recomputes the MAC in constant time and checks expiry.

use crate::error::StorageError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use doido_core::Result;
use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Whether a served file should render inline or download as an attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Render in the browser when possible.
    #[default]
    Inline,
    /// Force a download.
    Attachment,
}

impl Disposition {
    /// The `Content-Disposition` header value for `filename`.
    pub fn header(&self, filename: &str) -> String {
        let kind = match self {
            Disposition::Inline => "inline",
            Disposition::Attachment => "attachment",
        };
        format!("{kind}; filename=\"{}\"", filename.replace('"', ""))
    }
}

/// The signed payload carried inside a token.
#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    /// The signed data (e.g. a blob key).
    data: String,
    /// A namespace so a token signed for one purpose can't be replayed for another.
    purpose: String,
    /// Optional expiry, unix seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<u64>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Signs and verifies tokens with a secret key.
#[derive(Clone)]
pub struct Signer {
    secret: Vec<u8>,
}

impl Signer {
    /// Build a signer from raw secret bytes.
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    /// Read the secret from `DOIDO_SECRET_KEY_BASE`, falling back to an insecure
    /// development default (with a warning) when unset.
    pub fn from_env() -> Self {
        match std::env::var("DOIDO_SECRET_KEY_BASE") {
            Ok(s) if !s.is_empty() => Self::new(s.into_bytes()),
            _ => {
                doido_core::tracing::warn!(
                    "DOIDO_SECRET_KEY_BASE unset; using an insecure development signing key"
                );
                Self::new(b"doido-storage-development-secret".to_vec())
            }
        }
    }

    fn mac(&self, message: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts keys of any length");
        mac.update(message);
        mac.finalize().into_bytes().to_vec()
    }

    /// Sign `data` for `purpose`, optionally expiring after `expires_in`.
    pub fn sign(&self, data: &str, purpose: &str, expires_in: Option<Duration>) -> String {
        let payload = Payload {
            data: data.to_string(),
            purpose: purpose.to_string(),
            exp: expires_in.map(|d| now_secs() + d.as_secs()),
        };
        let json = serde_json::to_vec(&payload).expect("payload serializes");
        let message = B64.encode(&json);
        let sig = B64.encode(self.mac(message.as_bytes()));
        format!("{message}--{sig}")
    }

    /// Verify a token for `purpose`, returning the signed `data` or an error if
    /// the signature is invalid, the purpose mismatches, or it has expired.
    pub fn verify(&self, token: &str, purpose: &str) -> Result<String> {
        let (message, sig) = token
            .split_once("--")
            .ok_or_else(|| StorageError::InvalidSignature("malformed token".into()))?;

        let expected = self.mac(message.as_bytes());
        let provided = B64
            .decode(sig)
            .map_err(|_| StorageError::InvalidSignature("bad signature encoding".into()))?;
        // Constant-time comparison via HMAC's verify_slice.
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts keys of any length");
        mac.update(message.as_bytes());
        mac.verify_slice(&provided)
            .map_err(|_| StorageError::InvalidSignature("signature mismatch".into()))?;
        let _ = expected; // recomputed inside verify_slice; kept for clarity

        let json = B64
            .decode(message)
            .map_err(|_| StorageError::InvalidSignature("bad payload encoding".into()))?;
        let payload: Payload = serde_json::from_slice(&json)
            .map_err(|_| StorageError::InvalidSignature("bad payload".into()))?;

        if payload.purpose != purpose {
            return Err(StorageError::InvalidSignature("purpose mismatch".into()).into());
        }
        if let Some(exp) = payload.exp {
            if now_secs() > exp {
                return Err(StorageError::InvalidSignature("token expired".into()).into());
            }
        }
        Ok(payload.data)
    }
}
