use doido_jobs::{default_initializer, on_boot, reset_for_test};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn default_initializer_skips_in_migration_context() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    let hook_ran = Arc::new(AtomicUsize::new(0));
    let hook_ran_clone = hook_ran.clone();
    on_boot(move |_queue| async move {
        hook_ran_clone.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    std::env::set_var("DOIDO_COMMAND", "db");
    let result = default_initializer();
    assert!(result.is_ok());
    assert_eq!(hook_ran.load(Ordering::SeqCst), 0);

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
fn default_initializer_executes_on_boot_hooks() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset_for_test();

    let hook_ran = Arc::new(AtomicUsize::new(0));
    let hook_ran_clone = hook_ran.clone();
    on_boot(move |_queue| async move {
        hook_ran_clone.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    std::env::set_var("DOIDO_COMMAND", "server");
    let result = default_initializer();
    assert!(result.is_ok());
    assert_eq!(hook_ran.load(Ordering::SeqCst), 1);

    std::env::remove_var("DOIDO_COMMAND");
    reset_for_test();
}

fn dummy_job_one() {}
fn dummy_job_two() {}

#[test]
fn link_jobs_macro_compiles_and_retains_symbols() {
    doido_jobs::link_jobs!(dummy_job_one, dummy_job_two);
    doido_jobs::retain_jobs!(dummy_job_one);
}
