use doido_storage::{
    default_initializer, reset_for_test, try_storage, MemoryService, Signer, Storage,
};
use std::sync::{Arc, Mutex};

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
fn default_initializer_fails_fast_in_production_when_unreachable() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    if try_storage().is_none() {
        let prev_env = std::env::var("DOIDO_ENV").ok();
        std::env::set_var("DOIDO_ENV", "production");
        std::env::set_var("DOIDO_COMMAND", "server");

        let res = default_initializer();
        assert!(
            res.is_err(),
            "expected error in production when storage fails"
        );
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("storage initialization failed in production"));

        if let Some(val) = prev_env {
            std::env::set_var("DOIDO_ENV", val);
        } else {
            std::env::remove_var("DOIDO_ENV");
        }
        std::env::remove_var("DOIDO_COMMAND");
        reset_for_test();
    }
}

#[test]
fn default_initializer_warns_in_development_when_unreachable() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    if try_storage().is_none() {
        let prev_env = std::env::var("DOIDO_ENV").ok();
        std::env::set_var("DOIDO_ENV", "development");
        std::env::set_var("DOIDO_COMMAND", "server");

        let res = default_initializer();
        assert!(
            res.is_ok(),
            "expected Ok in development even if storage fails"
        );

        if let Some(val) = prev_env {
            std::env::set_var("DOIDO_ENV", val);
        } else {
            std::env::remove_var("DOIDO_ENV");
        }
        std::env::remove_var("DOIDO_COMMAND");
        reset_for_test();
    }
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

#[tokio::test]
async fn default_initializer_succeeds_when_storage_installed() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    // Install dummy storage if not present
    if try_storage().is_none() {
        let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let _ = doido_storage::global::init_with(Storage::new(
            conn,
            Arc::new(MemoryService::default()),
            Signer::new(b"secret".to_vec()),
        ))
        .await;
    }

    std::env::set_var("DOIDO_COMMAND", "server");
    let result = default_initializer();
    assert!(result.is_ok());
    assert!(try_storage().is_some());

    std::env::remove_var("DOIDO_COMMAND");
}
