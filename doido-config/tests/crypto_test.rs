use doido_config::crypto::{decrypt_credentials, encrypt_credentials, load_master_key};
use std::fs;
use tempfile::TempDir;

fn all_zeros_key() -> [u8; 32] {
    [0u8; 32]
}

#[test]
fn test_encrypt_decrypt_round_trip() {
    let key = all_zeros_key();
    let plaintext = "[database]\nurl = \"postgres://secret@host/db\"\n";
    let encrypted = encrypt_credentials(plaintext, &key).unwrap();
    let decrypted = decrypt_credentials(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_each_encryption_produces_unique_ciphertext() {
    let key = all_zeros_key();
    let c1 = encrypt_credentials("secret", &key).unwrap();
    let c2 = encrypt_credentials("secret", &key).unwrap();
    assert_ne!(c1, c2);
}

#[test]
fn test_decrypt_fails_with_wrong_key() {
    let key1 = [0u8; 32];
    let key2 = [1u8; 32];
    let encrypted = encrypt_credentials("secret", &key1).unwrap();
    let result = decrypt_credentials(&encrypted, &key2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("decryption failed"));
}

#[test]
fn test_decrypt_fails_on_garbage_input() {
    let key = all_zeros_key();
    let result = decrypt_credentials("not-base64!!!", &key);
    assert!(result.is_err());
}

#[test]
fn test_load_master_key_from_file() {
    let dir = TempDir::new().unwrap();
    let hex_key = "00".repeat(32);
    let key_path = dir.path().join("config/master.key");
    fs::create_dir_all(key_path.parent().unwrap()).unwrap();
    fs::write(&key_path, format!("{hex_key}\n")).unwrap();
    let key = load_master_key(dir.path()).unwrap();
    assert_eq!(key, [0u8; 32]);
}

#[test]
fn test_load_master_key_rejects_wrong_length() {
    let dir = TempDir::new().unwrap();
    let key_path = dir.path().join("config/master.key");
    fs::create_dir_all(key_path.parent().unwrap()).unwrap();
    fs::write(&key_path, "deadbeef").unwrap();
    if std::env::var("DOIDO_MASTER_KEY").is_err() {
        let result = load_master_key(dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("32 bytes"), "got: {msg}");
    }
}

#[test]
fn test_load_master_key_rejects_invalid_hex() {
    let dir = TempDir::new().unwrap();
    let key_path = dir.path().join("config/master.key");
    fs::create_dir_all(key_path.parent().unwrap()).unwrap();
    fs::write(&key_path, "not-valid-hex-string-at-all-!!!!").unwrap();
    if std::env::var("DOIDO_MASTER_KEY").is_err() {
        let result = load_master_key(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("valid hex"));
    }
}
