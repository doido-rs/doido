use axum::{
    routing::{get, post},
    Router,
};
use doido_controller::{csrf, MiddlewareStack};
use http::StatusCode;
use tower::ServiceExt;

fn app() -> Router {
    MiddlewareStack::new().with_csrf().apply(
        Router::new()
            .route("/", get(|| async { "ok" }))
            .route("/posts", post(|| async { "created" })),
    )
}

async fn post_status(cookie: Option<&str>, header: Option<&str>) -> StatusCode {
    let mut b = http::Request::builder().method("POST").uri("/posts");
    if let Some(c) = cookie {
        b = b.header("cookie", format!("csrf_token={c}"));
    }
    if let Some(h) = header {
        b = b.header("x-csrf-token", h);
    }
    let req = b.body(axum::body::Body::empty()).unwrap();
    app().oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn safe_get_needs_no_token() {
    let req = http::Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(app().oneshot(req).await.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn unsafe_post_without_token_is_forbidden() {
    assert_eq!(post_status(None, None).await, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unsafe_post_with_matching_token_passes() {
    let token = csrf::generate_token();
    assert_eq!(
        post_status(Some(&token), Some(&token)).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn unsafe_post_with_mismatched_token_is_forbidden() {
    let a = csrf::generate_token();
    let b = csrf::generate_token();
    assert_eq!(post_status(Some(&a), Some(&b)).await, StatusCode::FORBIDDEN);
}

#[test]
fn tokens_are_nonempty_and_unique() {
    let a = csrf::generate_token();
    let b = csrf::generate_token();
    assert!(!a.is_empty());
    assert_ne!(a, b, "each generated token is unique");
}
