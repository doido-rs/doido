use axum::body::Body;
use http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn redirect_route_301s_to_the_target() {
    let app = doido_controller::routes! {
        redirect!("/old", "/new");
    };

    let resp = app
        .oneshot(Request::builder().uri("/old").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(resp.headers().get("location").unwrap(), "/new");
}
