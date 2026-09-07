//! `Context::assign` merges shared variables into `render`, and the request flash
//! is auto-injected under the reserved `flash` key. Kept in its own test binary so
//! installing the process-global engine doesn't affect other tests.

use doido_controller::axum::body::Body;
use doido_controller::axum::response::Response;
use doido_controller::flash::Flash;
use doido_controller::session::CookieSessionStore;
use doido_controller::Context;
use http::Request;
use http_body_util::BodyExt;
use std::fs;
use std::sync::OnceLock;
use tempfile::TempDir;

/// One engine, installed once, holding every template these tests render. The
/// engine is a process global, so a per-test dir would race across parallel tests.
static ENGINE: OnceLock<TempDir> = OnceLock::new();

fn setup() {
    ENGINE.get_or_init(|| {
        let dir = TempDir::new().unwrap();
        let write = |tpl: &str, contents: &str| {
            let path = dir.path().join(format!("{tpl}.html.tera"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
        };
        write("page/index", "user={{ current_user }} title={{ title }}");
        write("plain/index", "<h1>{{ title }}</h1>");
        write("page/flash", "notice={{ flash.notice }}");
        doido_view::init(dir.path().to_str().unwrap()).unwrap();
        dir
    });
}

fn make_ctx() -> Context {
    let req = Request::builder().uri("/").body(()).unwrap();
    let (parts, _) = req.into_parts();
    Context::from_request_parts(parts)
}

async fn body_of(resp: Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn assigns_are_merged_and_data_wins_on_conflict() {
    setup();

    let mut ctx = make_ctx();
    ctx.assign("current_user", "alice");
    ctx.assign("title", "from-assign");

    // The per-render `data` overrides the assign for the same key.
    let resp = ctx.render("page/index", serde_json::json!({ "title": "from-data" }));
    assert_eq!(body_of(resp).await, "user=alice title=from-data");
}

#[tokio::test]
async fn render_without_assigns_passes_data_through() {
    setup();

    let ctx = make_ctx();
    let resp = ctx.render("plain/index", serde_json::json!({ "title": "Hello" }));
    assert_eq!(body_of(resp).await, "<h1>Hello</h1>");
}

#[tokio::test]
async fn flash_is_auto_injected_under_reserved_key() {
    setup();

    // A signed flash cookie the way the previous request would have written it.
    let store = CookieSessionStore::new(doido_controller::secret::key_base());
    let mut flash = Flash::new();
    flash.set("notice", "Saved!");
    let cookie = flash.to_cookie(&store);

    let req = Request::builder()
        .uri("/")
        .header("cookie", format!("_doido_flash={cookie}"))
        .body(Body::empty())
        .unwrap();
    let ctx = Context::build(req).await;

    // The action never touches `ctx.flash()`, yet the view still sees it.
    let resp = ctx.render("page/flash", serde_json::json!({}));
    assert_eq!(body_of(resp).await, "notice=Saved!");
}
