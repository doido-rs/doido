use doido_model::initializers::{
    default_initializer, detect_execution_context, init_for_context, ExecutionContext,
};
use doido_model::pool;

#[test]
fn default_initializer_detects_skip_for_migration_and_cli_commands() {
    let _lock = pool::test_lock();

    // Setting DOIDO_COMMAND to db / skip simulates `doido db migrate`
    std::env::set_var("DOIDO_COMMAND", "db");
    assert_eq!(detect_execution_context(), ExecutionContext::Skip);

    let res = default_initializer();
    assert!(res.is_ok(), "migration context must return Ok(())");

    // In skip context, pool is not initialized
    std::env::remove_var("DOIDO_COMMAND");
}

#[tokio::test]
async fn default_initializer_is_idempotent() {
    let _lock = pool::test_lock();

    // In test runner default (without DOIDO_COMMAND), it skips cleanly
    let res1 = default_initializer();
    assert!(res1.is_ok());

    let res2 = default_initializer();
    assert!(res2.is_ok());

    // When pool is installed, it is also idempotent
    if pool::try_pool().is_none() {
        let conn = pool::connect_with_url("sqlite::memory:").await.unwrap();
        let _ = pool::set_pool(conn);
    }

    assert!(pool::try_pool().is_some());
    let res3 = default_initializer();
    assert!(res3.is_ok());

    // Explicitly calling init_for_context with Server or Worker when pool is present returns Ok(())
    assert!(init_for_context(ExecutionContext::Server).is_ok());
    assert!(init_for_context(ExecutionContext::Worker).is_ok());
}

#[test]
fn detect_execution_context_recognizes_server_and_worker() {
    std::env::set_var("DOIDO_COMMAND", "server");
    assert_eq!(detect_execution_context(), ExecutionContext::Server);

    std::env::set_var("DOIDO_COMMAND", "worker");
    assert_eq!(detect_execution_context(), ExecutionContext::Worker);

    std::env::remove_var("DOIDO_COMMAND");
}

#[test]
fn run_async_executes_outside_tokio_runtime() {
    let res = doido_model::initializers::run_async(async { Ok(()) });
    assert!(res.is_ok());
}

#[tokio::test]
async fn run_async_executes_inside_tokio_runtime() {
    let res = doido_model::initializers::run_async(async { Ok(()) });
    assert!(res.is_ok());
}
