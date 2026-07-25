use axum::body::Body;
use axum::extract::Path;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn files(Path(path): Path<String>) -> String {
    format!("file: {path}")
}

#[tokio::test]
async fn glob_route_captures_the_remaining_path() {
    // `{*name}` is axum's catch-all segment; the routes! DSL passes it through.
    let app = doido_controller::routes! {
        get!("/files/{*path}", files);
    };

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/files/a/b/c.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"file: a/b/c.txt");
}
