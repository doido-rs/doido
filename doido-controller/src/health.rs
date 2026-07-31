//! Health-check endpoint (Rails 8 `GET /up`): returns 200 when the app boots.

use crate::axum::body::Body;
use crate::axum::response::Response;
use crate::axum::routing::get;
use crate::axum::Router;
use http::StatusCode;

/// The `/up` handler: a plain `200 OK`.
pub async fn up() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("OK"))
        .expect("valid health response")
}

/// Add the `GET /up` health route to a router.
pub fn with_health(router: Router) -> Router {
    router.route("/up", get(up))
}
