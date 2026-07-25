use crate::config::CorsConfig;
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::{from_fn, from_fn_with_state, Next},
    response::Response,
    Router,
};
use http::{header, HeaderValue, Method, StatusCode};
use std::sync::Arc;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
};

pub struct MiddlewareStack {
    cors: bool,
    cors_config: Option<CorsConfig>,
    allowed_hosts: Vec<String>,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self {
            cors: false,
            cors_config: None,
            allowed_hosts: Vec::new(),
        }
    }

    /// Enable permissive CORS (any origin/method/header). For fine-grained,
    /// config-driven CORS use [`with_cors_config`](Self::with_cors_config).
    pub fn with_cors(mut self) -> Self {
        self.cors = true;
        self
    }

    /// Enable CORS from parsed [`CorsConfig`] (spec 07 `[middleware.cors]`). A
    /// config with `enabled: false` is ignored, keeping CORS opt-in.
    pub fn with_cors_config(mut self, config: CorsConfig) -> Self {
        self.cors_config = Some(config);
        self
    }

    /// Restrict requests to an allowlist of `Host` header values (Rails
    /// `config.hosts` / `ActionDispatch::HostAuthorization`). An empty list
    /// permits any host; a request whose host is not listed gets a `403`.
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_hosts = hosts;
        self
    }

    pub fn apply(self, router: Router) -> Router {
        // Log every request and its response (method, path, status, latency)
        // through doido's centralized logger. Added after `CatchPanicLayer` so
        // it sits outermost and logs panic-recovered `500`s too.
        let mut r = router
            .layer(CatchPanicLayer::new())
            .layer(from_fn(crate::logging::log_requests));
        match &self.cors_config {
            Some(config) if config.enabled => r = r.layer(build_cors(config)),
            _ if self.cors => r = r.layer(CorsLayer::permissive()),
            _ => {}
        }
        if !self.allowed_hosts.is_empty() {
            r = r.layer(from_fn_with_state(Arc::new(self.allowed_hosts), host_guard));
        }
        r
    }
}

/// Reject requests whose `Host` header (port stripped) is not in the allowlist.
async fn host_guard(
    State(allowed): State<Arc<Vec<String>>>,
    request: Request,
    next: Next,
) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|h| h.split(':').next().unwrap_or(h).to_string());
    match host {
        Some(h) if allowed.iter().any(|a| a == &h) => next.run(request).await,
        _ => Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden host"))
            .expect("valid 403 response"),
    }
}

/// Build a [`CorsLayer`] from configuration. `"*"` in `allowed_origins` maps to
/// "any origin"; otherwise each origin/method is parsed and unparseable entries
/// are skipped.
fn build_cors(config: &CorsConfig) -> CorsLayer {
    let mut layer = CorsLayer::new();
    if config.allowed_origins.iter().any(|o| o == "*") {
        layer = layer.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        if !origins.is_empty() {
            layer = layer.allow_origin(origins);
        }
    }
    let methods: Vec<Method> = config
        .allowed_methods
        .iter()
        .filter_map(|m| Method::from_bytes(m.as_bytes()).ok())
        .collect();
    if !methods.is_empty() {
        layer = layer.allow_methods(methods);
    }
    layer
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}
