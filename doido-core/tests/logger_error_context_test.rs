//! ERROR events must include backtrace diagnostics — isolated binary (`Once` guard).

use doido_core::logger::{LogFormat, LoggerConfig};

#[test]
fn error_events_include_backtrace_context() {
    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("errors.log");
    let config = LoggerConfig {
        format: LogFormat::Compact,
        file: Some(log_path.to_string_lossy().into_owned()),
        ..LoggerConfig::default()
    };
    doido_core::logger::init_with_config(&config);
    doido_core::tracing::error!(error = "boom", "something failed");

    let rendered = std::fs::read_to_string(&log_path).unwrap();
    assert!(rendered.contains("something failed"));
    assert!(rendered.contains("backtrace:"));
}
