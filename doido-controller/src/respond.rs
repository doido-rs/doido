//! Format-based content negotiation (Rails `respond_to`).
//!
//! [`Context::negotiated_format`](crate::Context::negotiated_format) picks a
//! [`Format`] from the request (a `.json`/`.html` path extension wins, else the
//! `Accept` header), and [`Context::respond_to`](crate::Context::respond_to)
//! returns a [`RespondTo`] builder that runs the branch matching that format.

use crate::axum::body::Body;
use crate::axum::response::Response;
use http::StatusCode;

/// A negotiated response format. `Any` matches the first branch offered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Html,
    Json,
    Any,
}

/// Builder that evaluates only the branch matching the negotiated format.
pub struct RespondTo {
    format: Format,
    response: Option<Response>,
}

impl RespondTo {
    pub fn new(format: Format) -> Self {
        Self {
            format,
            response: None,
        }
    }

    /// Offer an HTML branch (runs for `Html` or `Any`).
    pub fn html(mut self, f: impl FnOnce() -> Response) -> Self {
        if self.response.is_none() && matches!(self.format, Format::Html | Format::Any) {
            self.response = Some(f());
        }
        self
    }

    /// Offer a JSON branch (runs for `Json` or `Any`).
    pub fn json(mut self, f: impl FnOnce() -> Response) -> Self {
        if self.response.is_none() && matches!(self.format, Format::Json | Format::Any) {
            self.response = Some(f());
        }
        self
    }

    /// Resolve to the matched branch, or `406 Not Acceptable` if none matched.
    pub fn finish(self) -> Response {
        self.response.unwrap_or_else(|| {
            Response::builder()
                .status(StatusCode::NOT_ACCEPTABLE)
                .body(Body::empty())
                .expect("valid 406 response")
        })
    }
}
