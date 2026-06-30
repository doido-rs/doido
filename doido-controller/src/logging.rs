//! Request/response logging middleware.
//!
//! [`log_requests`] logs each incoming request and its response through the
//! framework's centralized logger (`doido_core::logger`), so every HTTP exchange
//! flows through the same `tracing` subscriber as jobs, mail and ORM queries. It
//! is wired into the always-on [`MiddlewareStack`] (`crate::stack`).
//!
//! A per-request UUID (`request_id`) ties the `request` and `response` log lines
//! together; it is taken from an inbound `x-request-id` header when present (so
//! an upstream proxy's id is preserved) or generated otherwise, and echoed back
//! on the response so clients can correlate too. The `request` line carries the
//! method, path, query and request headers; the `response` line carries the
//! status, latency and response headers.

use axum::{extract::Request, middleware::Next, response::Response};
use doido_core::tracing::Instrument;
use http::{HeaderMap, HeaderName, HeaderValue};
use std::time::Instant;
use uuid::Uuid;

/// Header carrying the request correlation id, in/out.
const REQUEST_ID_HEADER: &str = "x-request-id";

/// Logs an incoming request and its response through the doido logger.
///
/// Two events flow through the global `tracing` subscriber per exchange, sharing
/// a `request_id`: a `request` line when it arrives (method, path, query, request
/// headers), and a `response` line once the response is ready (status, latency,
/// response headers). Both run inside a `request` span so nested events (e.g. SQL
/// queries) correlate back to the originating request.
pub async fn log_requests(request: Request, next: Next) -> Response {
    let request_id = resolve_request_id(request.headers());
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let query = request.uri().query().map(str::to_owned);
    let request_headers = format_headers(request.headers());

    // Span carrying the request identity; nested events inherit it.
    let span = doido_core::tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %method,
        path = %path,
    );
    doido_core::tracing::info!(
        target: doido_core::logger::REQUEST_TARGET,
        parent: &span,
        request_id = %request_id,
        method = %method,
        path = %path,
        query = query.as_deref().unwrap_or(""),
        headers = %request_headers,
        "request"
    );

    let start = Instant::now();
    // `instrument` enters the span for the whole handler, across `.await`s.
    let mut response = next.run(request).instrument(span.clone()).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    // Echo the correlation id back so clients/proxies can trace it; do this
    // before logging so the logged response headers reflect what's sent.
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), value);
    }

    let status = response.status().as_u16();
    let response_headers = format_headers(response.headers());
    {
        // Emit the response event inside the span; no `.await` follows.
        let _guard = span.enter();
        doido_core::tracing::info!(
            target: doido_core::logger::RESPONSE_TARGET,
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            latency_ms = latency_ms,
            headers = %response_headers,
            "response"
        );
    }

    response
}

/// Returns the inbound `x-request-id` when present and non-empty, otherwise a
/// freshly generated UUID v4.
fn resolve_request_id(headers: &HeaderMap) -> String {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Renders request headers as `name: value` pairs for a single log field.
/// Sensitive headers (auth, cookies) are redacted so secrets never reach logs.
fn format_headers(headers: &HeaderMap) -> String {
    headers
        .iter()
        .map(|(name, value)| {
            let rendered = if is_sensitive(name.as_str()) {
                "[redacted]"
            } else {
                value.to_str().unwrap_or("[non-utf8]")
            };
            format!("{}: {}", name.as_str(), rendered)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether a header's value must be redacted from logs. Header names compare
/// lowercase (the `http` crate normalizes them).
fn is_sensitive(name: &str) -> bool {
    matches!(
        name,
        "authorization" | "proxy-authorization" | "cookie" | "set-cookie"
    )
}
