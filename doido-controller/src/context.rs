use axum::{
    body::Body,
    extract::{FromRequestParts, RawPathParams, Request},
    http::{header, HeaderValue, StatusCode},
    response::Response,
};
use doido_model::sea_orm::DatabaseConnection;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Maximum request body size accepted by [`Context::form`]/[`Context::body_json`].
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

/// Per-request context passed to every action.
pub struct Context {
    pub(crate) parts: http::request::Parts,
    /// Taken (set to `None`) once read by [`form`](Self::form)/[`body_json`](Self::body_json).
    pub(crate) body: Option<Body>,
    /// Matched path parameters (e.g. `id` from `/posts/{id}`), in route order.
    pub(crate) path_params: Vec<(String, String)>,
}

impl Context {
    /// Central constructor used by the `#[controller]` macro. Splits the request,
    /// captures matched path params, and retains the body for later reads.
    pub async fn build(req: Request) -> Self {
        let (mut parts, body) = req.into_parts();
        let path_params = Self::extract_path_params(&mut parts).await;
        Self {
            parts,
            body: Some(body),
            path_params,
        }
    }

    async fn extract_path_params(parts: &mut http::request::Parts) -> Vec<(String, String)> {
        match RawPathParams::from_request_parts(parts, &()).await {
            Ok(params) => params
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn from_request_parts(parts: http::request::Parts) -> Self {
        Self {
            parts,
            body: None,
            path_params: Vec::new(),
        }
    }

    pub fn from_request(parts: http::request::Parts, body: Body) -> Self {
        Self {
            parts,
            body: Some(body),
            path_params: Vec::new(),
        }
    }

    /// The application's database connection (global pool installed at boot).
    pub fn db(&self) -> &'static DatabaseConnection {
        doido_model::pool::pool()
    }

    /// A matched path parameter by name, e.g. `ctx.param("id")` for `/posts/{id}`.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.path_params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Deserialize a URL-encoded (form) request body. Consumes the body.
    pub async fn form<T: DeserializeOwned>(&mut self) -> doido_core::Result<T> {
        let bytes = self.read_body().await?;
        serde_urlencoded::from_bytes(&bytes)
            .map_err(|e| doido_core::anyhow::anyhow!("form deserialization failed: {e}"))
    }

    /// Deserialize a JSON request body. Consumes the body.
    pub async fn body_json<T: DeserializeOwned>(&mut self) -> doido_core::Result<T> {
        let bytes = self.read_body().await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| doido_core::anyhow::anyhow!("JSON body deserialization failed: {e}"))
    }

    async fn read_body(&mut self) -> doido_core::Result<Vec<u8>> {
        let body = self
            .body
            .take()
            .ok_or_else(|| doido_core::anyhow::anyhow!("request body already consumed"))?;
        let bytes = axum::body::to_bytes(body, MAX_BODY_BYTES)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("failed to read request body: {e}"))?;
        Ok(bytes.to_vec())
    }

    /// Deserialize typed params from the request URI query string.
    pub fn params<T: serde::de::DeserializeOwned>(&self) -> doido_core::Result<T> {
        let query = self.parts.uri.query().unwrap_or("");
        serde_urlencoded::from_str(query)
            .map_err(|e| doido_core::anyhow::anyhow!("params deserialization failed: {e}"))
    }

    /// The query string as strong [`Params`](crate::params::Params), for
    /// `require`/`permit` allowlisting before use.
    pub fn query_params(&self) -> crate::params::Params {
        let query = self.parts.uri.query().unwrap_or("");
        let pairs: Vec<(String, String)> = serde_urlencoded::from_str(query).unwrap_or_default();
        let mut map = serde_json::Map::new();
        for (k, v) in pairs {
            map.insert(k, serde_json::Value::String(v));
        }
        crate::params::Params::new(serde_json::Value::Object(map))
    }

    /// Render a Tera view to an HTML 200 response.
    ///
    /// `template` is resolved by the global [`doido_view`] engine (installed at
    /// boot) against `app/views`, with the `.html.tera` suffix added — e.g.
    /// `"posts/index"` → `app/views/posts/index.html.tera`. A render failure (or
    /// an uninitialised engine) yields a `500`.
    pub fn render(&self, template: &str, data: serde_json::Value) -> Response {
        match doido_view::render(template, &data) {
            Ok(html) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(html))
                .expect("valid html response"),
            Err(error) => {
                tracing::error!(%error, template, "view render failed");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal Server Error"))
                    .expect("valid 500 response")
            }
        }
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

    /// The negotiated response [`Format`](crate::respond::Format): a `.json` or
    /// `.html` path extension wins, otherwise the `Accept` header is inspected;
    /// anything else is `Any`.
    pub fn negotiated_format(&self) -> crate::respond::Format {
        use crate::respond::Format;
        let path = self.parts.uri.path();
        if path.ends_with(".json") {
            return Format::Json;
        }
        if path.ends_with(".html") {
            return Format::Html;
        }
        match self
            .parts
            .headers
            .get(header::ACCEPT)
            .and_then(|a| a.to_str().ok())
        {
            Some(accept) if accept.contains("application/json") => Format::Json,
            Some(accept) if accept.contains("text/html") => Format::Html,
            _ => Format::Any,
        }
    }

    /// Begin format-based content negotiation (Rails `respond_to`).
    pub fn respond_to(&self) -> crate::respond::RespondTo {
        crate::respond::RespondTo::new(self.negotiated_format())
    }

    /// Whether the request's `If-None-Match` matches `etag` (or is `*`).
    pub fn etag_matches(&self, etag: &str) -> bool {
        match self
            .parts
            .headers
            .get(header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok())
        {
            Some("*") => true,
            Some(inm) => inm.split(',').map(str::trim).any(|t| t == etag),
            None => false,
        }
    }

    fn if_modified_since_matches(&self, last_modified: &str) -> bool {
        self.parts
            .headers
            .get(header::IF_MODIFIED_SINCE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v == last_modified)
            .unwrap_or(false)
    }

    /// HTTP conditional-GET check (Rails `fresh_when`): if the request's
    /// validators match `etag` (`If-None-Match`) or `last_modified`
    /// (`If-Modified-Since`), return a `304 Not Modified` echoing the validators;
    /// otherwise return `None` and render normally (setting the same validators).
    pub fn fresh_when(&self, etag: Option<&str>, last_modified: Option<&str>) -> Option<Response> {
        let fresh = etag.map(|e| self.etag_matches(e)).unwrap_or(false)
            || last_modified
                .map(|lm| self.if_modified_since_matches(lm))
                .unwrap_or(false);
        if !fresh {
            return None;
        }
        let mut builder = Response::builder().status(StatusCode::NOT_MODIFIED);
        if let Some(e) = etag {
            builder = builder.header(header::ETAG, e);
        }
        if let Some(lm) = last_modified {
            builder = builder.header(header::LAST_MODIFIED, lm);
        }
        Some(builder.body(Body::empty()).expect("valid 304 response"))
    }
}

/// Lets a `#[controller]` action body evaluate to either a [`Response`] or a
/// `Result<Response, E>`. The macro wraps every action body in
/// `into_action_response()`, so actions can use `?` for fallible work (DB calls,
/// body parsing) and an `Err` becomes a `500` response.
pub trait IntoActionResponse {
    fn into_action_response(self) -> Response;
}

impl IntoActionResponse for Response {
    fn into_action_response(self) -> Response {
        self
    }
}

impl<E: std::fmt::Display> IntoActionResponse for Result<Response, E> {
    fn into_action_response(self) -> Response {
        match self {
            Ok(response) => response,
            Err(error) => {
                tracing::error!(%error, "action returned an error");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal Server Error"))
                    .expect("static 500 response is valid")
            }
        }
    }
}

// `Context` must stay `Send` so controller handler futures (which hold a
// `&mut Context` across `.await`) satisfy axum's `Handler` bound.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Context>();
};
