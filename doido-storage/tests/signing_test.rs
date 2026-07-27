//! Signed tokens verify only with the right key, purpose, and before expiry.

use doido_storage::Signer;
use std::time::Duration;

#[test]
fn roundtrip_verifies() {
    let signer = Signer::new(b"secret".to_vec());
    let token = signer.sign("blob-key-123", "blob", None);
    assert_eq!(signer.verify(&token, "blob").unwrap(), "blob-key-123");
}

#[test]
fn wrong_purpose_rejected() {
    let signer = Signer::new(b"secret".to_vec());
    let token = signer.sign("k", "blob", None);
    assert!(signer.verify(&token, "disk_upload").is_err());
}

#[test]
fn tampered_token_rejected() {
    let signer = Signer::new(b"secret".to_vec());
    let token = signer.sign("k", "blob", None);
    let mut bad = token.clone();
    bad.push('x');
    assert!(signer.verify(&bad, "blob").is_err());
}

#[test]
fn wrong_key_rejected() {
    let a = Signer::new(b"secret-a".to_vec());
    let b = Signer::new(b"secret-b".to_vec());
    let token = a.sign("k", "blob", None);
    assert!(b.verify(&token, "blob").is_err());
}

#[test]
fn expired_token_rejected() {
    let signer = Signer::new(b"secret".to_vec());
    // Already expired (0-second lifetime, then the check is `now > exp`).
    let token = signer.sign("k", "blob", Some(Duration::from_secs(0)));
    std::thread::sleep(Duration::from_millis(1100));
    assert!(signer.verify(&token, "blob").is_err());
}
