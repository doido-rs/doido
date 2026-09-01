//! Direct `Context` API coverage (helpers, caching, cookies, flash commit paths).

use doido_controller::axum::body::Body;
use doido_controller::axum::response::Response;
use doido_controller::respond::Format;
use doido_controller::Context;
use http::{header, Request, StatusCode};
use http_body_util::BodyExt;

fn parts(uri: &str) -> http::request::Parts {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap()
        .into_parts()
        .0
}

#[tokio::test]
async fn context_response_helpers() {
    let ctx = Context::from_request_parts(parts("/posts.json?q=1"));
    let json = ctx.json(serde_json::json!({ "ok": true }));
    assert_eq!(json.status(), StatusCode::OK);
    assert_eq!(
        json.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let redirect = ctx.redirect_to("/welcome");
    assert_eq!(redirect.status(), StatusCode::FOUND);
    assert_eq!(
        redirect.headers().get(header::LOCATION).unwrap(),
        "/welcome"
    );

    assert_eq!(ctx.status(418).status(), StatusCode::from_u16(418).unwrap());
}

#[tokio::test]
async fn context_params_and_query() {
    let ctx = Context::from_request_parts(parts("/search?page=2&sort=asc"));
    #[derive(serde::Deserialize)]
    struct Q {
        page: String,
        sort: String,
    }
    let q: Q = ctx.params().unwrap();
    assert_eq!(q.page, "2");
    assert_eq!(q.sort, "asc");

    let params = ctx.query_params();
    assert_eq!(params.get("page").and_then(|v| v.as_str()), Some("2"));
}

#[tokio::test]
async fn context_negotiated_format_and_wants_json() {
    let html = Context::from_request_parts(parts("/posts.html"));
    assert_eq!(html.negotiated_format(), Format::Html);
    assert!(!html.wants_json());

    let json_path = Context::from_request_parts(parts("/posts.json"));
    assert_eq!(json_path.negotiated_format(), Format::Json);
    assert!(json_path.wants_json());

    let accept_json = Context::from_request_parts({
        let mut p = parts("/posts");
        p.headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        p
    });
    assert!(accept_json.wants_json());
}

#[tokio::test]
async fn context_is_json_request_reads_content_type() {
    let req = Request::builder()
        .method("POST")
        .uri("/api")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap();
    let ctx = Context::build(req).await;
    assert!(ctx.is_json_request());
}

#[tokio::test]
async fn context_fresh_when_and_etag() {
    let ctx = Context::from_request_parts({
        let mut p = parts("/asset");
        p.headers.insert(
            header::IF_NONE_MATCH,
            header::HeaderValue::from_static("\"v1\""),
        );
        p
    });
    assert!(ctx.etag_matches("\"v1\""));
    let not_modified = ctx.fresh_when(Some("\"v1\""), None).unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);

    let stale = Context::from_request_parts(parts("/asset"));
    assert!(stale.fresh_when(Some("\"v1\""), None).is_none());
}

#[tokio::test]
async fn context_send_data_and_send_file() {
    let ctx = Context::from_request_parts(parts("/download"));
    let resp = ctx.send_data(b"hello".to_vec(), "text/plain", Some("greeting.txt"));
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("greeting.txt"));

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("data.bin");
    std::fs::write(&path, b"bin").unwrap();
    let file_resp = ctx.send_file(&path, None).await.unwrap();
    let bytes = file_resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"bin");
}

#[tokio::test]
async fn context_cookies_commit_to_response() {
    let req = Request::builder()
        .uri("/")
        .header(header::COOKIE, "theme=dark")
        .body(Body::empty())
        .unwrap();
    let mut ctx = Context::build(req).await;
    assert_eq!(ctx.cookies().get("theme"), Some("dark"));
    ctx.cookies().set("lang", "en");
    ctx.cookies().set_signed("uid", "7");

    let mut resp = Response::new(Body::empty());
    ctx.commit_to_response(&mut resp);
    let cookies: Vec<_> = resp
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    assert!(cookies.iter().any(|c| c.starts_with("lang=en")));
    assert!(cookies.iter().any(|c| c.starts_with("uid=")));
}

#[tokio::test]
async fn context_flash_commit_clears_read_only_flash() {
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let mut ctx = Context::build(req).await;
    ctx.flash().set("notice", "saved");

    let mut resp = Response::new(Body::empty());
    ctx.commit_to_response(&mut resp);
    let set: Vec<_> = resp
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .collect();
    assert!(set.iter().any(|c| c.contains("_doido_flash=")));

    // Simulate next request carrying that flash cookie, read-only (no new flash set).
    let flash_cookie = set
        .iter()
        .find(|c| c.contains("_doido_flash="))
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let req2 = Request::builder()
        .uri("/")
        .header(header::COOKIE, flash_cookie)
        .body(Body::empty())
        .unwrap();
    let mut ctx2 = Context::build(req2).await;
    assert_eq!(ctx2.flash().get("notice"), Some("saved"));
    let mut resp2 = Response::new(Body::empty());
    ctx2.commit_to_response(&mut resp2);
    let swept: Vec<_> = resp2
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    assert!(swept
        .iter()
        .any(|c| c.contains("_doido_flash=") && c.contains("Max-Age=0")));
}

#[test]
fn into_action_response_maps_errors_to_500() {
    use doido_controller::IntoActionResponse;
    let ok = Response::new(Body::empty()).into_action_response();
    assert_eq!(ok.status(), StatusCode::OK);
    let err: Result<Response, &str> = Err("boom");
    assert_eq!(
        err.into_action_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}
