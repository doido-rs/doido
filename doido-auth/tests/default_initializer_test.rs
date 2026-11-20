use doido_auth::{default_initializer, reset_for_test, try_global};
use doido_model::testing::TestDb;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn default_initializer_skips_in_migration_context() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    std::env::set_var("DOIDO_COMMAND", "db");
    let result = default_initializer();
    assert!(result.is_ok());
    assert!(try_global().is_none());

    std::env::remove_var("DOIDO_COMMAND");
    reset_for_test();
}

#[test]
fn default_initializer_is_idempotent() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    std::env::set_var("DOIDO_COMMAND", "db");
    assert!(default_initializer().is_ok());
    assert!(default_initializer().is_ok());

    std::env::remove_var("DOIDO_COMMAND");
    reset_for_test();
}

#[test]
fn default_initializer_registers_locales_and_views() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    std::env::set_var("DOIDO_COMMAND", "db");
    let result = default_initializer();
    assert!(result.is_ok());

    // Verify i18n translation registered
    assert!(doido_core::i18n::locale_available("en"));

    std::env::remove_var("DOIDO_COMMAND");
    reset_for_test();
}

#[tokio::test]
async fn default_initializer_initializes_state_with_database() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    let db = TestDb::new().await.unwrap();
    let _ = doido_model::pool::set_pool(db.conn().clone());

    std::env::set_var("DOIDO_COMMAND", "server");
    let result = default_initializer();
    assert!(result.is_ok());
    assert!(try_global().is_some());

    std::env::remove_var("DOIDO_COMMAND");
    reset_for_test();
}
