//! Shared HMAC-SHA256 signing helpers used by the cookie jar (and available to
//! any value that needs tamper-evident transport). Signatures are base64url,
//! no-pad, and verified in constant time.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Sign `msg` with `secret`, returning the base64url signature.
pub fn sign(secret: &[u8], msg: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts a key of any length");
    mac.update(msg);
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

/// Verify a base64url `signature` over `msg` under `secret` (constant time).
pub fn verify(secret: &[u8], msg: &[u8], signature: &str) -> bool {
    let Ok(sig) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(msg);
    mac.verify_slice(&sig).is_ok()
}
