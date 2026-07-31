use doido_controller::axum::{routing::get, Router};
use doido_controller::config::Config;
use doido_controller::{MiddlewareStack, YamlConfig};
use http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn cors_enabled_from_config_sets_allow_origin() {
    let yaml = "middleware:\n  cors:\n    enabled: true\n    allowed_origins: [\"https://app.example\"]\n    allowed_methods: [\"GET\", \"POST\"]\n";
    let config = YamlConfig::from_yaml(yaml).unwrap();
    let stack = MiddlewareStack::new().with_cors_config(config.middleware().cors.clone());
    let app = stack.apply(Router::new().route("/", get(|| async { "ok" })));

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
