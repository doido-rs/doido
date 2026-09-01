use doido_controller::axum::{routing::get, Router};
use doido_controller::config::CorsConfig;
use doido_controller::MiddlewareStack;
use http::{Method, Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn force_ssl_redirects_insecure_requests() {
    let app = MiddlewareStack::new()
        .with_force_ssl()
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .method(Method::GET)
        .uri("http://example.com/welcome")
        .header("host", "example.com")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        resp.headers().get("location").unwrap().to_str().unwrap(),
        "https://example.com/welcome"
    );
}

#[tokio::test]
async fn force_ssl_allows_forwarded_https() {
    let app = MiddlewareStack::new()
        .with_force_ssl()
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("/")
        .header("x-forwarded-proto", "https")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn csrf_blocks_post_without_matching_token() {
    let app = MiddlewareStack::new()
        .with_csrf()
        .apply(Router::new().route("/", get(|| async { "ok" }).post(|| async { "created" })));
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn csrf_skipped_in_api_only_mode() {
    let app = MiddlewareStack::new()
        .with_api_only(true)
        .with_csrf()
        .apply(Router::new().route("/", get(|| async { "ok" }).post(|| async { "created" })));
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn allowed_hosts_rejects_unknown_host() {
    let app = MiddlewareStack::new()
        .with_allowed_hosts(vec!["allowed.test".into()])
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("/")
        .header("host", "evil.test")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cors_permissive_allows_any_origin() {
    let app = MiddlewareStack::new()
        .with_cors()
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("origin", "https://any.test")
        .header("access-control-request-method", "GET")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn allowed_hosts_permits_listed_host() {
    let app = MiddlewareStack::new()
        .with_allowed_hosts(vec!["allowed.test".into()])
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .uri("/")
        .header("host", "allowed.test")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn insert_before_and_after_transforms_router() {
    use doido_controller::axum::routing::get;
    let app = MiddlewareStack::new()
        .insert_before(|r| r.route("/inner", get(|| async { "inner" })))
        .insert_after(|r| r.route("/outer", get(|| async { "outer" })))
        .apply(Router::new().route("/", get(|| async { "root" })));
    let _ = app;
}

#[tokio::test]
async fn cors_config_layer_applies_when_enabled() {
    let config = CorsConfig {
        enabled: true,
        allowed_origins: vec!["https://app.test".into()],
        allowed_methods: vec!["GET".into()],
        allowed_headers: vec!["content-type".into()],
        allow_credentials: false,
    };
    let app = MiddlewareStack::new()
        .with_cors_config(config)
        .apply(Router::new().route("/", get(|| async { "ok" })));
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("origin", "https://app.test")
        .header("access-control-request-method", "GET")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
