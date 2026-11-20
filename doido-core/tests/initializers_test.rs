use doido_core::initializers::{
    cli_context, detect_execution_context, is_test_runner, run_async, ExecutionContext,
    Initializers,
};
use doido_core::Environment;
use std::sync::{Arc, Mutex};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn execution_context_predicates() {
    assert!(ExecutionContext::Server.is_server());
    assert!(!ExecutionContext::Server.is_worker());
    assert!(!ExecutionContext::Server.is_skip());
    assert!(ExecutionContext::Server.is_server_or_worker());
    assert!(!ExecutionContext::Server.is_migration());

    assert!(ExecutionContext::Worker.is_worker());
    assert!(!ExecutionContext::Worker.is_server());
    assert!(!ExecutionContext::Worker.is_skip());
    assert!(ExecutionContext::Worker.is_server_or_worker());
    assert!(!ExecutionContext::Worker.is_migration());

    assert!(ExecutionContext::Skip.is_skip());
    assert!(!ExecutionContext::Skip.is_server());
    assert!(!ExecutionContext::Skip.is_worker());
    assert!(!ExecutionContext::Skip.is_server_or_worker());
    assert!(ExecutionContext::Skip.is_migration());
}

#[test]
fn cli_context_matches_detect_execution_context() {
    assert_eq!(cli_context(), detect_execution_context());
}

#[test]
fn run_async_executes_outside_tokio_runtime() {
    let res = run_async(async { Ok("success") });
    assert_eq!(res.unwrap(), "success");
}

#[tokio::test]
async fn run_async_executes_inside_tokio_runtime() {
    let res = run_async(async { Ok(12345) });
    assert_eq!(res.unwrap(), 12345);
}

#[test]
fn test_runner_detected_via_environment() {
    let _lock = ENV_LOCK.lock().unwrap();

    let prev_env = std::env::var("DOIDO_ENV").ok();

    std::env::set_var("DOIDO_ENV", "test");
    assert!(Environment::get_env().is_test());
    assert!(is_test_runner());

    if let Some(val) = prev_env {
        std::env::set_var("DOIDO_ENV", val);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}

#[test]
fn detect_execution_context_recognizes_modes() {
    let _lock = ENV_LOCK.lock().unwrap();

    std::env::set_var("DOIDO_COMMAND", "server");
    assert_eq!(detect_execution_context(), ExecutionContext::Server);

    std::env::set_var("DOIDO_COMMAND", "worker");
    assert_eq!(detect_execution_context(), ExecutionContext::Worker);

    std::env::set_var("DOIDO_COMMAND", "db");
    assert_eq!(detect_execution_context(), ExecutionContext::Skip);

    std::env::remove_var("DOIDO_COMMAND");
}

#[test]
fn initializers_run_in_registration_order() {
    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let mut inits = Initializers::new();
    let a = order.clone();
    inits.register("first", move || {
        a.lock().unwrap().push("first");
        Ok(())
    });
    let b = order.clone();
    inits.register("second", move || {
        b.lock().unwrap().push("second");
        Ok(())
    });

    assert_eq!(inits.names(), vec!["first", "second"]);
    inits.run_all().unwrap();
    assert_eq!(*order.lock().unwrap(), vec!["first", "second"]);
}

#[test]
fn a_failing_initializer_stops_the_sequence() {
    let mut inits = Initializers::new();
    inits.register("boom", || Err(doido_core::anyhow::anyhow!("nope")));
    inits.register("never", || panic!("should not run"));
    assert!(inits.run_all().is_err());
}

#[test]
fn initializers_with_defaults_includes_core() {
    let inits = Initializers::with_defaults();
    assert!(inits.names().contains(&"core"));
    assert!(inits.run_all().is_ok());
}
