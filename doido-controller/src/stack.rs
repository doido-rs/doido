use axum::Router;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, trace::TraceLayer};

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
        let mut r = router
            .layer(CatchPanicLayer::new())
            .layer(TraceLayer::new_for_http());
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
