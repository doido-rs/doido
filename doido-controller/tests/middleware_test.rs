use axum::{routing::get, Router};
use doido_controller::{CookieSessionStore, MiddlewareStack, SessionStore};
use http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_middleware_stack_processes_request() {
    let app = MiddlewareStack::new().apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_logging_middleware_passes_response_through_unchanged() {
    // The request/response logging middleware must be transparent: the handler's
    // status and body reach the client untouched while the exchange is logged.
    let handler = || async { (StatusCode::IM_A_TEAPOT, "brewing") };
    let app = MiddlewareStack::new().apply(Router::new().route("/", get(handler)));
    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"brewing");
}

#[tokio::test]
async fn test_logging_middleware_sets_request_id_header() {
    // With no inbound id, the middleware generates one and echoes it back so the
    // request and response log lines can be correlated by the same value.
    let app = MiddlewareStack::new().apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let id = resp
        .headers()
        .get("x-request-id")
        .expect("x-request-id header present");
    assert!(!id.to_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_logging_middleware_preserves_inbound_request_id() {
    // An upstream proxy's id must survive end to end.
    let app = MiddlewareStack::new().apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("/")
        .header("x-request-id", "upstream-123")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let id = resp
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(id, "upstream-123");
}

#[tokio::test]
async fn test_cookie_session_store_load_returns_none() {
    let store = CookieSessionStore;
    assert!(store.load("any-id").await.unwrap().is_none());
}

#[tokio::test]
async fn test_session_store_is_object_safe() {
    let store: Box<dyn SessionStore> = Box::new(CookieSessionStore);
    assert!(store.load("x").await.unwrap().is_none());
}
