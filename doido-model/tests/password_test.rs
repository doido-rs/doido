use doido_model::password::{
    generate_token, hash_password_with_cost, verify_password, HasSecurePassword,
};

// Low bcrypt cost keeps the test fast; production uses hash_password (DEFAULT_COST).
const TEST_COST: u32 = 4;

#[test]
fn hashing_produces_a_verifiable_non_plaintext_digest() {
    let digest = hash_password_with_cost("s3cret", TEST_COST).unwrap();
    assert_ne!(digest, "s3cret", "the password is not stored in plaintext");
    assert!(
        verify_password("s3cret", &digest),
        "correct password verifies"
    );
    assert!(
        !verify_password("wrong", &digest),
        "wrong password is rejected"
    );
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
    let user = User {
        password_digest: hash_password_with_cost("hunter2", TEST_COST).unwrap(),
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
