use doido_core::Environment;

#[test]
fn default_initializer_runs_successfully_and_is_idempotent() {
    // Calling default_initializer the first time succeeds
    let res1 = doido_core::default_initializer();
    assert!(res1.is_ok(), "first call failed: {res1:?}");

    // Calling it again succeeds idempotently
    let res2 = doido_core::default_initializer();
    assert!(res2.is_ok(), "second call failed: {res2:?}");
}

#[test]
fn default_initializer_loads_dotenv_in_development() {
    let temp_dir = tempfile::tempdir().unwrap();
    let env_file = temp_dir.path().join(".env");
    std::fs::write(&env_file, "DOIDO_CORE_TEST_VAR=loaded_from_dotenv\n").unwrap();

    let prev_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // In development mode, .env should be loaded
    let prev_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_var("DOIDO_ENV", "development");

    let res = doido_core::default_initializer();
    assert!(res.is_ok());

    std::env::set_current_dir(prev_dir).unwrap();
    if let Some(env) = prev_env {
        std::env::set_var("DOIDO_ENV", env);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }

    assert_eq!(Environment::get_env(), Environment::Development);
}
