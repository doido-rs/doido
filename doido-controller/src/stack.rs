use axum::{middleware::from_fn, Router};
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};

pub struct MiddlewareStack {
    cors: bool,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self { cors: false }
    }

    pub fn with_cors(mut self) -> Self {
        self.cors = true;
        self
    }

    pub fn apply(self, router: Router) -> Router {
        // Log every request and its response (method, path, status, latency)
        // through doido's centralized logger. Added after `CatchPanicLayer` so
        // it sits outermost and logs panic-recovered `500`s too.
        let mut r = router
            .layer(CatchPanicLayer::new())
            .layer(from_fn(crate::logging::log_requests));
        if self.cors {
            r = r.layer(CorsLayer::permissive());
        }
        r
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}
