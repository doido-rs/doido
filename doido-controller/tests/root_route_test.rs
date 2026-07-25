use axum::body::Body;
use http::{Request, StatusCode};
use tower::ServiceExt;

async fn home() -> &'static str {
    "home"
}

#[tokio::test]
async fn root_route_serves_the_slash_path() {
    let app = doido_controller::routes! {
        root!(home);
        get!("/about", home);
    };

    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // A non-root path still 404s (root is only `/`).
    let missing = app
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
