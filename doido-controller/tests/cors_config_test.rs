use doido_controller::axum::{routing::get, Router};
use doido_controller::config::Config;
use doido_controller::{MiddlewareStack, YamlConfig};
use http::{Method, Request, StatusCode};
use tower::ServiceExt;

fn cors_app(yaml: &str) -> Router {
    let config = YamlConfig::from_yaml(yaml).unwrap();
    MiddlewareStack::new()
        .with_cors_config(config.middleware().cors.clone())
        .apply(Router::new().route("/", get(|| async { "ok" })))
}

#[tokio::test]
async fn cors_enabled_from_config_sets_allow_origin() {
    let yaml = "middleware:\n  cors:\n    enabled: true\n    allowed_origins: [\"https://app.example\"]\n    allowed_methods: [\"GET\", \"POST\"]\n";
    let app = cors_app(yaml);

    let req = Request::builder()
        .uri("/")
        .header("origin", "https://app.example")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let allow = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("configured CORS layer sets the allow-origin header");
    assert_eq!(allow, "https://app.example");
}

#[tokio::test]
async fn cors_disabled_by_default() {
    let config = YamlConfig::from_yaml("server:\n  bind: 0.0.0.0\n  port: 3000\n").unwrap();
    assert!(
        !config.middleware().cors.enabled,
        "CORS is opt-in: off unless configured"
    );
}

#[tokio::test]
async fn cors_permissive_defaults_answer_preflight_with_auth_headers() {
    // Fivia-like config: enabled, wildcard origin, empty method/header lists.
    let yaml = "middleware:\n  cors:\n    enabled: true\n    allowed_origins: [\"*\"]\n    allowed_methods: []\n    allowed_headers: []\n";
    let app = cors_app(yaml);

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("origin", "http://localhost:3001")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "authorization, content-type")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NO_CONTENT,
        "preflight should succeed, got {}",
        resp.status()
    );
    assert_eq!(
        resp.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("*")
    );
    let allow_methods = resp
        .headers()
        .get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .expect("empty allowed_methods should default to any method");
    assert!(allow_methods.contains('*') || allow_methods.contains("POST"));
    let allow_headers = resp
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .expect("empty allowed_headers should default to any header");
    assert!(
        allow_headers.contains('*')
            || (allow_headers.contains("authorization") && allow_headers.contains("content-type")),
        "expected authorization and content-type in allow-headers, got {allow_headers}"
    );
}

#[tokio::test]
async fn cors_explicit_headers_list() {
    let yaml = "middleware:\n  cors:\n    enabled: true\n    allowed_origins: [\"https://app.example\"]\n    allowed_methods: [\"POST\"]\n    allowed_headers: [\"authorization\", \"content-type\"]\n";
    let app = cors_app(yaml);

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("origin", "https://app.example")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "authorization, content-type")
        .body(doido_controller::axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let allow_headers = resp
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .expect("explicit allowed_headers should be advertised");
    assert!(
        allow_headers.contains("authorization") && allow_headers.contains("content-type"),
        "got {allow_headers}"
    );
}
