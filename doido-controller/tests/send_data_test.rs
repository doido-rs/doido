use doido_controller::Context;
use http::StatusCode;
use http_body_util::BodyExt;

fn ctx() -> Context {
    Context::from_request_parts(
        http::Request::builder()
            .uri("/")
            .body(())
            .unwrap()
            .into_parts()
            .0,
    )
}

#[tokio::test]
async fn send_data_sets_download_headers_and_body() {
    let resp = ctx().send_data(b"hello".to_vec(), "text/plain", Some("greeting.txt"));
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
    assert_eq!(
        resp.headers().get("content-disposition").unwrap(),
        "attachment; filename=\"greeting.txt\""
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"hello");
}

#[tokio::test]
async fn send_file_returns_file_contents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.bin");
    std::fs::write(&path, b"filedata").unwrap();

    let resp = ctx()
        .send_file(&path, Some("application/octet-stream"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"filedata");
}

#[tokio::test]
async fn send_file_missing_path_errors() {
    assert!(ctx().send_file("/no/such/file", None).await.is_err());
}
