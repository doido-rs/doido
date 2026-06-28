use axum::Router;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::CorsLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

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
        // Log every request and response at INFO (method, path, status, latency)
        // through the centralized subscriber installed by `doido_core::logger`.
        let trace = TraceLayer::new_for_http()
            .on_request(DefaultOnRequest::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO));

        let mut r = router.layer(CatchPanicLayer::new()).layer(trace);
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
