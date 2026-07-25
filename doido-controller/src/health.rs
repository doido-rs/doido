//! Health-check endpoint (Rails 8 `GET /up`): returns 200 when the app boots.

use axum::body::Body;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
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
