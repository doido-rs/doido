//! File-backed logger init — isolated binary (`Once` guard).

use doido_core::logger::{LogFormat, LoggerConfig};

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
