use doido_model::password::{
    configure_password_hashing, generate_token, hash_password, hash_password_with_config,
    hash_password_with_cost, needs_rehash, verify_password, HasSecurePassword, PasswordHashConfig,
};

const TEST_COST: u32 = 4;

#[test]
fn hash_password_uses_argon2id() {
    configure_password_hashing(PasswordHashConfig::fast_test(), None, 128);
    let digest = hash_password("secret").unwrap();
    assert!(digest.starts_with("$argon2id$"));
    assert!(verify_password("secret", &digest));
}

#[test]
fn verify_password_rejects_malformed_digest() {
    assert!(!verify_password("secret", "not-a-valid-digest"));
}

#[test]
fn bcrypt_legacy_verify_and_needs_rehash() {
    configure_password_hashing(PasswordHashConfig::fast_test(), None, 128);
    let digest = hash_password_with_cost("s3cret", TEST_COST).unwrap();
    assert!(digest.starts_with("$2"));
    assert!(verify_password("s3cret", &digest));
    assert!(!verify_password("wrong", &digest));
    assert!(needs_rehash(&digest));
}

#[test]
fn argon2_digest_does_not_need_rehash_when_params_match() {
    let config = PasswordHashConfig::fast_test();
    configure_password_hashing(config, None, 128);
    let digest = hash_password_with_config("hunter2", config).unwrap();
    assert!(!needs_rehash(&digest));
}

struct User {
    password_digest: String,
}
impl HasSecurePassword for User {
    fn password_digest(&self) -> &str {
        &self.password_digest
    }
}

#[test]
fn authenticate_checks_the_digest() {
    configure_password_hashing(PasswordHashConfig::fast_test(), None, 128);
    let user = User {
        password_digest: hash_password_with_config("hunter2", PasswordHashConfig::fast_test())
            .unwrap(),
    };
    assert!(user.authenticate("hunter2"));
    assert!(!user.authenticate("nope"));
}

#[test]
fn generated_tokens_are_nonempty_and_unique() {
    let a = generate_token();
    let b = generate_token();
    assert!(!a.is_empty());
    assert_ne!(a, b);
}

#[test]
fn pepper_changes_the_digest() {
    configure_password_hashing(
        PasswordHashConfig::fast_test(),
        Some(b"pepper".to_vec()),
        128,
    );
    let with_pepper = hash_password("same").unwrap();
    configure_password_hashing(PasswordHashConfig::fast_test(), None, 128);
    let without = hash_password("same").unwrap();
    assert_ne!(with_pepper, without);
    configure_password_hashing(
        PasswordHashConfig::fast_test(),
        Some(b"pepper".to_vec()),
        128,
    );
    assert!(verify_password("same", &with_pepper));
    configure_password_hashing(PasswordHashConfig::fast_test(), None, 128);
}
