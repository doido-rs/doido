use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn dashboard() -> &'static str {
    "dashboard"
}

#[tokio::test]
async fn mount_nests_a_sub_router_under_a_prefix() {
    let admin = doido_controller::routes! {
        get!("/dashboard", dashboard);
    };
    let app = doido_controller::routes! {
        mount!("/admin", admin);
    };

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"dashboard");
}
