//! JsonResponse logger init — isolated binary (`Once` guard).

use doido_core::logger::{LogFormat, LoggerConfig};

#[test]
fn init_with_json_response_format() {
    let config = LoggerConfig {
        format: LogFormat::JsonResponse,
        ..LoggerConfig::default()
    };
    doido_core::logger::init_with_config(&config);
    doido_core::tracing::info!(target: doido_core::logger::RESPONSE_TARGET, status = 200u16, "ok");
}
