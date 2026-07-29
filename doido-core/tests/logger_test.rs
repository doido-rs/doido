//! Installs the global subscriber, so kept in its own test binary to avoid
//! clashing with other tests that set up tracing.

use doido_core::logger::{LogFormat, LoggerConfig, REQUEST_TARGET, RESPONSE_TARGET};

#[test]
fn init_is_idempotent_and_emits() {
    // Calling more than once must not panic.
    doido_core::logger::init();
    doido_core::logger::init();

    // The subscriber is now installed; emitting an event must not panic either.
    doido_core::tracing::info!("logger smoke test");
}

#[test]
fn init_with_custom_directives() {
    doido_core::logger::init_with("warn");
    doido_core::tracing::warn!("init_with smoke");
}

#[test]
fn init_with_config_verbose_and_file() {
    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("nested/app.log");
    let config = LoggerConfig {
        format: LogFormat::Verbose,
        file: Some(log_path.to_string_lossy().into_owned()),
        ..LoggerConfig::default()
    };
    doido_core::logger::init_with_config(&config);
    doido_core::tracing::info!("file logger smoke");
    assert!(log_path.exists());
}

#[test]
fn request_and_response_targets_are_stable() {
    assert_eq!(REQUEST_TARGET, "doido::request");
    assert_eq!(RESPONSE_TARGET, "doido::response");
}
