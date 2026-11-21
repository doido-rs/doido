use doido_mailer::{default_initializer, reset_for_test};
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn default_initializer_skips_when_context_is_skip() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    std::env::set_var("DOIDO_COMMAND", "db");
    let result = default_initializer();
    assert!(result.is_ok());

    std::env::remove_var("DOIDO_COMMAND");
}

#[test]
fn default_initializer_is_idempotent() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    std::env::set_var("DOIDO_COMMAND", "db");
    assert!(default_initializer().is_ok());
    assert!(default_initializer().is_ok());

    std::env::remove_var("DOIDO_COMMAND");
}

#[test]
fn default_initializer_succeeds_for_server() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    std::env::set_var("DOIDO_COMMAND", "server");
    let result = default_initializer();
    assert!(result.is_ok());

    std::env::remove_var("DOIDO_COMMAND");
}
