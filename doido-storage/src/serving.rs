//! Axum handlers that serve blobs and accept direct uploads — the analogue of
//! ActiveStorage's `blobs`/`disk`/`direct_uploads` controllers.
//!
//! [`routes`] returns a `Router` (with [`Storage`] as state) mounting, under the
//! configured prefix (default `/doido/storage`):
//!
//! * `GET  {prefix}/blobs/redirect/{signed_id}/{filename}` — 302 to the service's
//!   native URL (S3/Azure) or to the proxy route (disk/memory).
//! * `GET  {prefix}/blobs/proxy/{signed_id}/{filename}` — stream the object.
//! * `PUT  {prefix}/disk/{token}` — receive a disk/memory direct upload.
//! * `POST {prefix}/direct_uploads` — create a blob and return an upload URL.

use crate::blob::{self, Blob};
use crate::client::{Storage, PURPOSE_DISK_UPLOAD};
use crate::content_type;
use crate::signing::Disposition;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Build the storage router mounted at the facade's prefix.
pub fn routes(storage: Storage) -> Router {
    let p = storage.prefix().to_string();
    Router::new()
        .route(
            &format!("{p}/blobs/redirect/{{signed_id}}/{{filename}}"),
            get(redirect_handler),
        )
        .route(
            &format!("{p}/blobs/proxy/{{signed_id}}/{{filename}}"),
            get(proxy_handler),
        )
        .route(&format!("{p}/disk/{{token}}"), put(disk_upload_handler))
        .route(&format!("{p}/direct_uploads"), post(direct_uploads_handler))
        .with_state(storage)
}

async fn resolve_blob(storage: &Storage, signed_id: &str) -> Option<Blob> {
    let key = storage.verify_signed_id(signed_id).ok()?;
    storage.find_blob(&key).await.ok().flatten()
}

async fn redirect_handler(
    State(storage): State<Storage>,
    Path((signed_id, _filename)): Path<(String, String)>,
) -> Response {
    let Some(blob) = resolve_blob(&storage, &signed_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match storage.url_for(&blob, Disposition::Inline).await {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn proxy_handler(
    State(storage): State<Storage>,
    Path((signed_id, _filename)): Path<(String, String)>,
) -> Response {
    let Some(blob) = resolve_blob(&storage, &signed_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let bytes = match storage.download(&blob.key).await {
        Ok(b) => b,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let ct = blob
        .content_type
        .clone()
        .unwrap_or_else(|| content_type::DEFAULT.to_string());
    (
        [
            (header::CONTENT_TYPE, ct),
            (
                header::CONTENT_DISPOSITION,
                Disposition::Inline.header(&blob.filename),
            ),
        ],
        bytes,
    )
        .into_response()
}

async fn disk_upload_handler(
    State(storage): State<Storage>,
    Path(token): Path<String>,
    body: Bytes,
) -> Response {
    let key = match storage.signer().verify(&token, PURPOSE_DISK_UPLOAD) {
        Ok(k) => k,
        Err(_) => return StatusCode::FORBIDDEN.into_response(),
    };
    match storage.service().upload(&key, body.to_vec(), None).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Client-provided details for a direct upload.
#[derive(Debug, Deserialize)]
pub struct DirectUploadRequest {
    /// Original filename.
    pub filename: String,
    /// Declared byte size (recorded on the blob; verified on upload later).
    #[serde(default)]
    pub byte_size: Option<i64>,
    /// Declared base64 MD5 checksum.
    #[serde(default)]
    pub checksum: Option<String>,
    /// Declared content type (else inferred from the filename).
    #[serde(default)]
    pub content_type: Option<String>,
}

/// The upload target returned to the client.
#[derive(Debug, Serialize)]
pub struct DirectUpload {
    /// URL to PUT the file bytes to.
    pub url: String,
    /// Headers the client must send with the PUT.
    pub headers: HashMap<String, String>,
}

/// The direct-upload response: the blob's signed id plus its upload target.
#[derive(Debug, Serialize)]
pub struct DirectUploadResponse {
    /// The blob storage key.
    pub key: String,
    /// The blob's signed id (attach with this after uploading).
    pub signed_id: String,
    /// Where and how to upload the bytes.
    pub direct_upload: DirectUpload,
}

async fn direct_uploads_handler(
    State(storage): State<Storage>,
    Json(req): Json<DirectUploadRequest>,
) -> Response {
    let content_type = req
        .content_type
        .or_else(|| content_type::from_filename(&req.filename).map(str::to_string));

    let blob = Blob {
        key: blob::new_key(),
        filename: req.filename.clone(),
        content_type,
        metadata: None,
        service_name: storage.service().name().to_string(),
        byte_size: req.byte_size.unwrap_or(0),
        checksum: req.checksum,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    if blob::insert(storage.conn(), &blob).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let opts = crate::service::UrlOptions {
        expires_in: storage.expires_in(),
        disposition: Disposition::Inline,
        filename: Some(blob.filename.clone()),
        content_type: blob.content_type.clone(),
    };

    // Prefer the backend's native presigned PUT; fall back to the disk route.
    let url = match storage.service().presigned_put(&blob.key, &opts).await {
        Ok(Some(u)) => u,
        _ => {
            let token =
                storage
                    .signer()
                    .sign(&blob.key, PURPOSE_DISK_UPLOAD, Some(storage.expires_in()));
            format!("{}/disk/{}", storage.prefix(), token)
        }
    };

    let mut headers = HashMap::new();
    if let Some(ct) = &blob.content_type {
        headers.insert("Content-Type".to_string(), ct.clone());
    }

    Json(DirectUploadResponse {
        signed_id: storage.signed_id(&blob.key),
        key: blob.key,
        direct_upload: DirectUpload { url, headers },
    })
    .into_response()
}
