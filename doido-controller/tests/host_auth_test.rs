use axum::{routing::get, Router};
use doido_controller::MiddlewareStack;
use http::{Request, StatusCode};
use tower::ServiceExt;

fn app(hosts: &[&str]) -> Router {
    let stack =
        MiddlewareStack::new().with_allowed_hosts(hosts.iter().map(|h| h.to_string()).collect());
    stack.apply(Router::new().route("/", get(|| async { "ok" })))
}

async fn status_for(hosts: &[&str], host_header: Option<&str>) -> StatusCode {
    let mut b = Request::builder().uri("/");
    if let Some(h) = host_header {
        b = b.header("host", h);
    }
    let req = b.body(axum::body::Body::empty()).unwrap();
    app(hosts).oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn allowed_host_passes() {
    assert_eq!(
        status_for(&["example.com"], Some("example.com")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn allowed_host_ignores_port() {
    assert_eq!(
        status_for(&["example.com"], Some("example.com:3000")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn disallowed_host_is_forbidden() {
    assert_eq!(
        status_for(&["example.com"], Some("evil.test")).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn empty_allowlist_permits_any_host() {
    assert_eq!(status_for(&[], Some("anything.test")).await, StatusCode::OK);
}
