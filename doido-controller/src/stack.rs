use crate::config::CorsConfig;
use axum::{middleware::from_fn, Router};
use http::{HeaderValue, Method};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
};

pub struct MiddlewareStack {
    cors: bool,
    cors_config: Option<CorsConfig>,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self {
            cors: false,
            cors_config: None,
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
        r
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
