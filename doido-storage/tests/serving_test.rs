//! The axum serving routes: proxy streams bytes, redirect 307s, and a direct
//! upload creates a blob then accepts its bytes over the disk PUT route.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use doido_storage::{MemoryService, Signer, Storage};
use std::sync::Arc;
use tower::ServiceExt;

async fn storage() -> Storage {
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let s = Storage::new(
        conn,
        Arc::new(MemoryService::default()),
        Signer::new(b"test-secret".to_vec()),
    );
    s.ensure_tables().await.unwrap();
    s
}

#[tokio::test]
async fn proxy_streams_bytes() {
    let s = storage().await;
    let blob = s
        .create_and_upload("hi.txt", b"proxied!".to_vec(), None)
        .await
        .unwrap();
    let path = s.proxy_path(&blob);

    let app = doido_storage::serving::routes(s);
    let resp = app
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/plain"
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"proxied!");
}

#[tokio::test]
async fn redirect_points_at_proxy_for_memory() {
    let s = storage().await;
    let blob = s
        .create_and_upload("hi.txt", b"x".to_vec(), None)
        .await
        .unwrap();
    let path = s.redirect_path(&blob);

    let app = doido_storage::serving::routes(s);
    let resp = app
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.contains("/blobs/proxy/"), "location was {loc}");
}

#[tokio::test]
async fn bad_signature_is_not_found() {
    let s = storage().await;
    let app = doido_storage::serving::routes(s);
    let resp = app
        .oneshot(
            Request::get("/doido/storage/blobs/proxy/not-a-valid-token/x.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn direct_upload_creates_blob_then_accepts_bytes() {
    let s = storage().await;
    let app = doido_storage::serving::routes(s.clone());

    // 1. Request a direct upload.
    let req_body = serde_json::json!({ "filename": "up.txt", "content_type": "text/plain" });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/doido/storage/direct_uploads")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let key = json["key"].as_str().unwrap().to_string();
    let url = json["direct_upload"]["url"].as_str().unwrap().to_string();
    assert!(url.contains("/disk/"), "url was {url}");

    // 2. PUT the bytes to the returned URL.
    let resp = app
        .oneshot(
            Request::put(&url)
                .body(Body::from(b"uploaded bytes".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // 3. The bytes are now retrievable through the service.
    assert_eq!(s.download(&key).await.unwrap(), b"uploaded bytes");
}
