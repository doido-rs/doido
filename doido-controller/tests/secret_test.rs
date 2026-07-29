//! Process-global secret key base (`OnceLock` — one test binary, one lifecycle).

use doido_controller::secret;

#[test]
fn key_base_lifecycle() {
    assert_eq!(
        secret::key_base(),
        b"doido-dev-insecure-secret-key-base-change-me".to_vec()
    );

    assert!(secret::set_key_base(b"prod-secret".to_vec()).is_ok());
    assert_eq!(secret::key_base(), b"prod-secret");

    assert_eq!(
        secret::set_key_base(b"other-secret".to_vec()),
        Err(b"other-secret".to_vec())
    );
    assert_eq!(secret::key_base(), b"prod-secret");
}
