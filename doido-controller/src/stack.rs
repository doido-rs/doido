use crate::config::CorsConfig;
use crate::axum::{
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

/// A router transformation registered by the app to insert its own middleware
/// relative to doido's always-on stack (Rails `config.middleware.insert_*`).
type RouterTransform = Box<dyn FnOnce(Router) -> Router + Send>;

pub struct MiddlewareStack {
    cors: bool,
    cors_config: Option<CorsConfig>,
    allowed_hosts: Vec<String>,
    csrf: bool,
    force_ssl: bool,
    before: Vec<RouterTransform>,
    after: Vec<RouterTransform>,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self {
            cors: false,
            cors_config: None,
            allowed_hosts: Vec::new(),
            csrf: false,
            force_ssl: false,
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    /// Enable CSRF protection (double-submit cookie): state-changing requests
    /// must send an `X-CSRF-Token` header matching the `csrf_token` cookie.
    pub fn with_csrf(mut self) -> Self {
        self.csrf = true;
        self
    }

    /// Redirect insecure requests to HTTPS (Rails `force_ssl`). A request is
    /// considered secure when its `X-Forwarded-Proto` is `https` (or the URI
    /// scheme is `https`); otherwise it gets a 301 to the `https://` URL.
    pub fn with_force_ssl(mut self) -> Self {
        self.force_ssl = true;
        self
    }

    /// Insert custom middleware **inside** the always-on layers (closer to the
    /// router), i.e. it runs after logging/panic-recovery on the way in. The
    /// closure receives the router and returns it with its `.layer(...)` added.
    pub fn insert_before(
        mut self,
        transform: impl FnOnce(Router) -> Router + Send + 'static,
    ) -> Self {
        self.before.push(Box::new(transform));
        self
    }

    /// Insert custom middleware **outside** the always-on layers (outermost), so
    /// it wraps the whole stack.
    pub fn insert_after(
        mut self,
        transform: impl FnOnce(Router) -> Router + Send + 'static,
    ) -> Self {
        self.after.push(Box::new(transform));
        self
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
        // App-registered "before" middleware sits innermost (closest to routes).
        let mut r = router;
        for transform in self.before {
            r = transform(r);
        }
        // Log every request and its response (method, path, status, latency)
        // through doido's centralized logger. Added after `CatchPanicLayer` so
        // it sits outermost and logs panic-recovered `500`s too.
        r = r
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
        if self.csrf {
            r = r.layer(from_fn(csrf_guard));
        }
        if self.force_ssl {
            r = r.layer(from_fn(force_ssl_guard));
        }
        // App-registered "after" middleware wraps everything (outermost).
        for transform in self.after {
            r = transform(r);
        }
        r
    }
}

/// Redirect insecure requests to their `https://` equivalent with a 301.
async fn force_ssl_guard(request: Request, next: Next) -> Response {
    let forwarded_https = request
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|p| p.eq_ignore_ascii_case("https"))
        .unwrap_or(false);
    let scheme_https = request.uri().scheme_str() == Some("https");
    if forwarded_https || scheme_https {
        return next.run(request).await;
    }

    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let path = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let location = format!("https://{host}{path}");

    match HeaderValue::from_str(&location) {
        Ok(value) => {
            let mut response = Response::builder()
                .status(StatusCode::MOVED_PERMANENTLY)
                .body(Body::empty())
                .expect("valid 301 response");
            response.headers_mut().insert(header::LOCATION, value);
            response
        }
        Err(_) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .expect("valid 400 response"),
    }
}

/// Enforce the double-submit CSRF check on state-changing methods. Safe methods
/// (GET/HEAD/OPTIONS/TRACE) always pass; others require the `csrf_token` cookie
/// and the `X-CSRF-Token` header to be present and equal.
async fn csrf_guard(request: Request, next: Next) -> Response {
    let method = request.method();
    let is_safe = matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    );
    if is_safe {
        return next.run(request).await;
    }

    let cookie_token = request
        .headers()
        .get(header::COOKIE)
        .and_then(|c| c.to_str().ok())
        .and_then(crate::csrf::token_from_cookie_header);
    let header_token = request
        .headers()
        .get("x-csrf-token")
        .and_then(|h| h.to_str().ok())
        .map(str::to_string);

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if crate::csrf::tokens_match(&cookie, &header) => {
            next.run(request).await
        }
        _ => Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("CSRF token mismatch"))
            .expect("valid 403 response"),
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
