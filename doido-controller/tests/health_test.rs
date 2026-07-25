use axum::Router;
use doido_controller::health::with_health;
use doido_controller::testing::send;
use http::StatusCode;

#[tokio::test]
async fn up_endpoint_returns_200_ok() {
    let app = with_health(Router::new());
    let resp = send(app, "GET", "/up", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, "OK");
}
