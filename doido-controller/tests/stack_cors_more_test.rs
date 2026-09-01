//! Extra `MiddlewareStack` coverage: the credentialed CORS branches
//! (`mirror_request` for origin/methods/headers), the CSRF "safe method" and
//! "matching token" pass paths, and the `force_ssl` https-scheme pass path.

use doido_controller::axum::{routing::get, Router};
use doido_controller::config::CorsConfig;
use doido_controller::MiddlewareStack;
use http::{Method, Request, StatusCode};
use tower::ServiceExt;

fn body() -> doido_controller::axum::body::Body {
    doido_controller::axum::body::Body::empty()
}

#[tokio::test]
async fn cors_wildcard_with_credentials_mirrors_origin() {
    let config = CorsConfig {
        enabled: true,
        allowed_origins: vec!["*".into()],
        allowed_methods: vec!["*".into()],
        allowed_headers: vec!["*".into()],
        allow_credentials: true,
    };
    let app = MiddlewareStack::new()
        .with_cors_config(config)
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("origin", "https://client.test")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "authorization")
        .body(body())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // With credentials, a wildcard mirrors the concrete request origin rather
    // than echoing `*` (which browsers reject alongside credentials).
    assert_eq!(
        resp.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://client.test")
    );
    assert_eq!(
        resp.headers()
            .get("access-control-allow-credentials")
            .and_then(|v| v.to_str().ok()),
        Some("true")
    );
}

#[tokio::test]
async fn cors_all_invalid_entries_advertise_nothing() {
    // Non-wildcard lists whose every entry fails to parse leave the dimension
    // unset on the layer (the `(!x.is_empty()).then(..)` → None branch).
    let config = CorsConfig {
        enabled: true,
        allowed_origins: vec!["::not a url::".into()],
        allowed_methods: vec!["".into()],
        allowed_headers: vec!["not a header name!!".into()],
        allow_credentials: false,
    };
    let app = MiddlewareStack::new()
        .with_cors_config(config)
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("origin", "https://client.test")
        .header("access-control-request-method", "POST")
        .body(body())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert!(resp
        .headers()
        .get("access-control-allow-origin")
        .is_none());
    assert!(resp
        .headers()
        .get("access-control-allow-methods")
        .is_none());
}

#[tokio::test]
async fn csrf_allows_safe_get_request() {
    let app = MiddlewareStack::new()
        .with_csrf()
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let resp = app
        .oneshot(Request::builder().uri("/").body(body()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn csrf_allows_post_with_matching_token() {
    let token = doido_controller::csrf::generate_token();
    let app = MiddlewareStack::new()
        .with_csrf()
        .apply(Router::new().route("/", get(|| async { "ok" }).post(|| async { "created" })));
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("cookie", format!("csrf_token={token}"))
        .header("x-csrf-token", &token)
        .body(body())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn force_ssl_allows_https_scheme_uri() {
    let app = MiddlewareStack::new()
        .with_force_ssl()
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("https://example.com/")
        .header("host", "example.com")
        .body(body())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
