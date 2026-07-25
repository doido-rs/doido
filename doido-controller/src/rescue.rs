//! `rescue_from`: map specific error types to responses instead of a blanket
//! `500` (Rails `rescue_from RecordNotFound, with: :not_found`).
//!
//! Actions return `Result<Response, anyhow::Error>`; on an `Err`, the registered
//! handlers are tried in order by downcasting the error to each handler's type.
//! The first that matches produces the response; if none match, the caller falls
//! back to the default `500`.

use axum::response::Response;
use doido_core::anyhow;

type Handler = Box<dyn Fn(&anyhow::Error) -> Option<Response> + Send + Sync>;

/// An ordered set of typed error handlers.
#[derive(Default)]
pub struct RescueHandlers {
    handlers: Vec<Handler>,
}

impl RescueHandlers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for error type `E`. When a rescued error downcasts to
    /// `E`, `handler` renders the response.
    pub fn on<E>(mut self, handler: impl Fn(&E) -> Response + Send + Sync + 'static) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.handlers
            .push(Box::new(move |err| err.downcast_ref::<E>().map(&handler)));
        self
    }

    /// Try each handler in registration order; return the first match, if any.
    pub fn rescue(&self, err: &anyhow::Error) -> Option<Response> {
        self.handlers.iter().find_map(|h| h(err))
    }
}
