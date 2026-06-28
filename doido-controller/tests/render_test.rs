//! `Context::render` end-to-end against the global Tera engine. Kept in its own
//! test binary so installing the process-global engine doesn't affect other
//! tests.

use doido_controller::Context;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::fs;
use tempfile::TempDir;

fn make_ctx() -> Context {
    let req = Request::builder().uri("/").body(()).unwrap();
    let (parts, _) = req.into_parts();
    Context::from_request_parts(parts)
}

#[tokio::test]
async fn render_uses_global_tera_engine() {
    // Lay out a views dir and install the engine over it.
    let dir = TempDir::new().unwrap();
    let tpl = dir.path().join("posts/index.html.tera");
    fs::create_dir_all(tpl.parent().unwrap()).unwrap();
    fs::write(&tpl, "<h1>{{ title }}</h1>").unwrap();
    doido_view::init(dir.path().to_str().unwrap()).unwrap();

    let ctx = make_ctx();
    let resp = ctx.render("posts/index", serde_json::json!({ "title": "Hello" }));

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap();
    assert!(ct.to_str().unwrap().contains("text/html"));
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(std::str::from_utf8(&body).unwrap(), "<h1>Hello</h1>");
}
