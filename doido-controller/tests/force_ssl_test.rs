use axum::{routing::get, Router};
use doido_controller::MiddlewareStack;
use http::StatusCode;
use tower::ServiceExt;

fn app() -> Router {
    MiddlewareStack::new()
        .with_force_ssl()
        .apply(Router::new().route("/dashboard", get(|| async { "ok" })))
}

#[tokio::test]
async fn insecure_request_is_redirected_to_https() {
    let req = http::Request::builder()
        .uri("/dashboard")
        .header("host", "example.com")
        .header("x-forwarded-proto", "http")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        resp.headers().get("location").unwrap(),
        "https://example.com/dashboard"
    );
}

#[tokio::test]
async fn secure_request_passes_through() {
    let req = http::Request::builder()
        .uri("/dashboard")
        .header("host", "example.com")
        .header("x-forwarded-proto", "https")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
