use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::Response,
};
use serde::Serialize;

/// Per-request context passed to every action.
pub struct Context {
    pub(crate) parts: http::request::Parts,
    #[allow(dead_code)] // reserved for body-param deserialization (future use)
    pub(crate) body: Body,
}

impl Context {
    pub fn from_request_parts(parts: http::request::Parts) -> Self {
        Self {
            parts,
            body: Body::empty(),
        }
    }

    pub fn from_request(parts: http::request::Parts, body: Body) -> Self {
        Self { parts, body }
    }

    /// Deserialize typed params from the request URI query string.
    pub fn params<T: serde::de::DeserializeOwned>(&self) -> doido_core::Result<T> {
        let query = self.parts.uri.query().unwrap_or("");
        serde_urlencoded::from_str(query)
            .map_err(|e| doido_core::anyhow::anyhow!("params deserialization failed: {e}"))
    }

    /// Return a plain-text 200 response (placeholder until doido-view is wired).
    pub fn render(&self, template: &str, _data: serde_json::Value) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(format!("render:{template}")))
            .unwrap()
    }

    /// Return a JSON 200 response.
    pub fn json<T: Serialize>(&self, data: T) -> Response {
        let body = serde_json::to_vec(&data).unwrap_or_default();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    /// Return a 302 redirect.
    pub fn redirect_to(&self, location: impl AsRef<str>) -> Response {
        Response::builder()
            .status(StatusCode::FOUND)
            .header(
                header::LOCATION,
                HeaderValue::from_str(location.as_ref()).unwrap(),
            )
            .body(Body::empty())
            .unwrap()
    }

    /// Return a response with an explicit status code and empty body.
    /// `code` must be a valid HTTP status code (100–999).
    pub fn status(&self, code: u16) -> Response {
        Response::builder()
            .status(code)
            .body(Body::empty())
            .unwrap()
    }

    /// Get a request header by name (lowercase).
    pub fn header(&self, name: &str) -> Option<&http::HeaderValue> {
        self.parts.headers.get(name)
    }
}
