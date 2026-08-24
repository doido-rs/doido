//! Development-only diagnostic error pages (Rails `ActionDispatch::DebugExceptions`).
//!
//! Active only when [`Environment::Development`](doido_core::Environment::Development)
//! is selected via `DOIDO_ENV`. HTML clients receive a styled page with the error
//! message, backtrace, and request context; production/test keep plain responses.

use crate::axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use doido_core::Environment;
use http::request::Parts;
use std::any::Any;
use std::backtrace::Backtrace;
use std::fmt::Display;

const ERROR_TEMPLATE: &str = include_str!("../templates/development/error.html");

/// Diagnostic metadata attached to error responses for the development middleware.
#[derive(Clone, Debug)]
pub struct DevelopmentErrorContext {
    pub status: u16,
    pub title: String,
    pub message: String,
    pub backtrace: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub query: Option<String>,
}

impl DevelopmentErrorContext {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            title: status_title(status),
            message: message.into(),
            backtrace: Some(format_backtrace(&Backtrace::force_capture())),
            method: None,
            path: None,
            query: None,
        }
    }

    pub fn with_request(mut self, parts: &Parts) -> Self {
        self.method = Some(parts.method.to_string());
        self.path = Some(parts.uri.path().to_owned());
        self.query = parts.uri.query().map(str::to_owned);
        self
    }

    pub fn with_request_info(
        mut self,
        method: impl Into<String>,
        path: impl Into<String>,
        query: Option<String>,
    ) -> Self {
        self.method = Some(method.into());
        self.path = Some(path.into());
        self.query = query;
        self
    }

    pub fn without_backtrace(mut self) -> Self {
        self.backtrace = None;
        self
    }
}

/// Whether the current process is running in the development environment.
pub fn is_development() -> bool {
    Environment::get_env() == Environment::Development
}

/// Whether the client expects an HTML response (mirrors [`crate::respond::Format`]).
pub fn wants_html_response(headers: &HeaderMap, path: &str) -> bool {
    if path.ends_with(".json") {
        return false;
    }
    if path.ends_with(".html") {
        return true;
    }
    match headers.get(header::ACCEPT).and_then(|a| a.to_str().ok()) {
        Some(accept) if accept.contains("application/json") => false,
        Some(accept) if accept.contains("text/html") => true,
        _ => true,
    }
}

/// Store diagnostic context on a response for the development middleware to render.
pub fn attach_error_context(response: &mut Response, context: DevelopmentErrorContext) {
    response.extensions_mut().insert(context);
}

/// Build a placeholder error response with attached diagnostic context.
pub fn development_error_response(
    status: StatusCode,
    message: impl Into<String>,
    parts: Option<&Parts>,
) -> Response {
    let mut context = DevelopmentErrorContext::new(status.as_u16(), message);
    if let Some(parts) = parts {
        context = context.with_request(parts);
    }
    let mut response = Response::builder()
        .status(status)
        .body(Body::from(placeholder_body(status)))
        .expect("valid error response");
    attach_error_context(&mut response, context);
    response
}

/// Render the development diagnostic HTML page.
pub fn render_development_error_page(context: &DevelopmentErrorContext) -> Response {
    let backtrace = context
        .backtrace
        .as_deref()
        .filter(|b| !b.is_empty())
        .unwrap_or("(backtrace unavailable)");

    let request_info = format_request_info(context);

    let html = ERROR_TEMPLATE
        .replace("{{ status }}", &context.status.to_string())
        .replace("{{ title }}", &escape_html(&context.title))
        .replace("{{ message }}", &escape_html(&context.message))
        .replace("{{ backtrace }}", &escape_html(backtrace))
        .replace("{{ request_info }}", &escape_html(&request_info));

    Response::builder()
        .status(context.status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(html))
        .expect("valid development error page")
}

/// Middleware that replaces 4xx/5xx placeholder responses with the diagnostic page.
pub async fn development_error_page_middleware(request: Request, next: Next) -> Response {
    if !is_development() {
        return next.run(request).await;
    }

    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let query = request.uri().query().map(str::to_owned);
    let wants_html = wants_html_response(request.headers(), &path);

    let mut response = next.run(request).await;

    if !wants_html || response.status().is_success() {
        return response;
    }

    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }

    if let Some(context) = take_error_context(&mut response) {
        let context = context.with_request_info(method, path, query);
        return render_development_error_page(&context);
    }

    if let Some(inferred) =
        infer_context_from_body(status, &mut response, &method, &path, query).await
    {
        return render_development_error_page(&inferred);
    }

    response
}

/// Custom panic handler for development: attach diagnostic context to the 500.
pub fn development_panic_response(err: Box<dyn Any + Send + 'static>) -> Response {
    let message = panic_message(&err);
    tracing::error!(%message, "request panicked");
    development_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("panic: {message}"),
        None,
    )
}

fn take_error_context(response: &mut Response) -> Option<DevelopmentErrorContext> {
    response.extensions_mut().remove()
}

async fn infer_context_from_body(
    status: StatusCode,
    response: &mut Response,
    method: &str,
    path: &str,
    query: Option<String>,
) -> Option<DevelopmentErrorContext> {
    let body_bytes = crate::axum::body::to_bytes(std::mem::take(response.body_mut()), 64 * 1024)
        .await
        .ok()?;
    let body = std::str::from_utf8(&body_bytes).unwrap_or("");

    if !is_generic_error_body(status, body) {
        *response.body_mut() = Body::from(body_bytes);
        return None;
    }

    let message = if body.is_empty() {
        status_message(status)
    } else {
        body.to_string()
    };

    Some(
        DevelopmentErrorContext::new(status.as_u16(), message)
            .with_request_info(method, path, query)
            .without_backtrace(),
    )
}

fn is_generic_error_body(status: StatusCode, body: &str) -> bool {
    if body.is_empty() {
        return true;
    }
    matches!(
        body,
        "Internal Server Error" | "CSRF token mismatch" | "Forbidden host" | "Not Found"
    ) || (status == StatusCode::NOT_FOUND && body.len() < 128)
}

fn status_title(status: u16) -> String {
    match status {
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ if (400..500).contains(&status) => "Client Error",
        _ if (500..600).contains(&status) => "Server Error",
        _ => "Error",
    }
    .to_string()
}

fn status_message(status: StatusCode) -> String {
    match status {
        StatusCode::NOT_FOUND => "No route matches this URL".to_string(),
        StatusCode::FORBIDDEN => "Forbidden".to_string(),
        StatusCode::INTERNAL_SERVER_ERROR => "Internal Server Error".to_string(),
        _ => status.canonical_reason().unwrap_or("Error").to_string(),
    }
}

fn placeholder_body(status: StatusCode) -> &'static str {
    match status {
        StatusCode::NOT_FOUND => "Not Found",
        StatusCode::FORBIDDEN => "Forbidden",
        _ => "Internal Server Error",
    }
}

fn format_request_info(context: &DevelopmentErrorContext) -> String {
    let method = context.method.as_deref().unwrap_or("?");
    let path = context.path.as_deref().unwrap_or("/");
    let query = context
        .query
        .as_deref()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();
    format!("{method} {path}{query}")
}

fn format_backtrace(backtrace: &Backtrace) -> String {
    backtrace.to_string()
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn panic_message(err: &Box<dyn Any + Send + 'static>) -> String {
    if let Some(msg) = err.downcast_ref::<&str>() {
        return (*msg).to_string();
    }
    if let Some(msg) = err.downcast_ref::<String>() {
        return msg.clone();
    }
    "unknown panic".to_string()
}

/// Log and build a development-aware error response for action/render failures.
pub fn log_and_error_response<E: Display>(
    status: StatusCode,
    error: E,
    parts: Option<&Parts>,
) -> Response {
    tracing::error!(%error, "request failed");
    if is_development() {
        development_error_response(status, error.to_string(), parts)
    } else {
        Response::builder()
            .status(status)
            .body(Body::from(placeholder_body(status)))
            .expect("valid error response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_page_includes_status_message_and_backtrace() {
        let context = DevelopmentErrorContext::new(500, "database connection failed")
            .with_request_info("GET", "/posts", None);
        let response = render_development_error_page(&context);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn escape_html_neutralizes_markup() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn wants_html_prefers_json_when_accept_says_so() {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        assert!(!wants_html_response(&headers, "/posts"));
    }

    #[test]
    fn wants_html_defaults_to_html_for_full_stack_apps() {
        assert!(wants_html_response(&HeaderMap::new(), "/posts"));
    }
}
