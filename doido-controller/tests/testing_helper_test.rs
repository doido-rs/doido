use axum::{routing::get, routing::post, Router};
use doido_controller::testing::send;
use http::StatusCode;

#[tokio::test]
async fn send_drives_a_router_and_captures_the_response() {
    let app = Router::new()
        .route("/", get(|| async { "home" }))
        .route("/echo", post(|body: String| async move { body }));

    let home = send(app.clone(), "GET", "/", "").await;
    assert_eq!(home.status, StatusCode::OK);
    assert_eq!(home.body, "home");

    let echo = send(app, "POST", "/echo", "ping").await;
    assert_eq!(echo.body, "ping");
}
