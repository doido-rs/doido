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
async fn test_cookie_session_store_load_returns_none() {
    let store = CookieSessionStore;
    assert!(store.load("any-id").await.unwrap().is_none());
}

#[tokio::test]
async fn test_session_store_is_object_safe() {
    let store: Box<dyn SessionStore> = Box::new(CookieSessionStore);
    assert!(store.load("x").await.unwrap().is_none());
}
