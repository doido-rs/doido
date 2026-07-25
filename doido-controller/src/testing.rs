//! Integration/system test helpers: drive a `Router` in-process and inspect the
//! response (Rails integration/system tests). Pair with `doido_model` factories
//! and `TestDb` for full-stack tests.

use axum::body::Body;
use axum::Router;
use http::{Request, StatusCode};
use tower::ServiceExt;

/// The status + body captured from a test request.
pub struct TestResponse {
    pub status: StatusCode,
    pub body: String,
}

/// Send a request to `router` in-process and capture the response.
pub async fn send(router: Router, method: &str, uri: &str, body: &str) -> TestResponse {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::from(body.to_string()))
        .expect("valid test request");
    let response = router
        .oneshot(request)
        .await
        .expect("router handled the request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    TestResponse {
        status,
        body: String::from_utf8_lossy(&bytes).to_string(),
    }
}
