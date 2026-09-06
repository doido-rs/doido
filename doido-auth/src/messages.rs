//! Localized auth flash/JSON messages via [`doido_core::i18n`].

use doido_controller::axum::body::Body;
use doido_controller::axum::http::header;
use doido_controller::axum::response::Response;

/// Translate a stable message key using the process locale (`DOIDO_LOCALE` / `en`).
pub fn t(key: &str) -> String {
    doido_core::i18n::translate_for(key, None)
}

/// JSON response with an explicit status and `{ "error": message }` body.
pub fn json_error(code: u16, message: impl Into<String>) -> Response {
    let body =
        serde_json::to_vec(&serde_json::json!({ "error": message.into() })).unwrap_or_default();
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}
