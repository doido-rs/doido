use doido_middleware::{MiddlewareStack, SessionStore, CookieSessionStore};
use axum::{Router, routing::get};
use tower::ServiceExt;
use http::{Request, StatusCode};

#[tokio::test]
async fn test_middleware_stack_processes_request() {
    let app = MiddlewareStack::new().apply(
        Router::new().route("/", get(|| async { "ok" }))
    );
    let req = Request::builder().uri("/").body(axum::body::Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cookie_session_store_load_returns_none() {
    let store = CookieSessionStore;
    assert!(store.load("any-id").unwrap().is_none());
}

#[tokio::test]
async fn test_session_store_is_object_safe() {
    let store: Box<dyn SessionStore> = Box::new(CookieSessionStore);
    assert!(store.load("x").unwrap().is_none());
}
