//! Session/flash/cookies surfaced on `Context` and flushed to `Set-Cookie` by
//! the `#[controller]` macro. Exercises the full two-request round trip.

use doido_controller::axum::body::Body;
use http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

struct SessionController;

#[doido_controller::controller]
impl SessionController {
    // Increment a session counter, and stash a flash message for the next request.
    async fn bump(ctx: Context) -> doido_controller::Response {
        let n: i64 = ctx.session().get("n").unwrap_or(0);
        ctx.session().set("n", n + 1);
        ctx.flash().set("notice", "bumped");
        ctx.json(serde_json::json!({ "n": n + 1 }))
    }

    // Read the session counter and the (one-shot) flash message.
    async fn read(ctx: Context) -> doido_controller::Response {
        let n: i64 = ctx.session().get("n").unwrap_or(0);
        let notice = ctx.flash().get("notice").unwrap_or("none").to_string();
        ctx.json(serde_json::json!({ "n": n, "notice": notice }))
    }
}

fn app() -> doido_controller::axum::Router {
    doido_controller::axum::Router::new()
        .route(
            "/bump",
            doido_controller::axum::routing::get(SessionController::bump),
        )
        .route(
            "/read",
            doido_controller::axum::routing::get(SessionController::read),
        )
}

/// Collapse a response's `Set-Cookie` headers into a `Cookie` request header.
fn cookies_from(resp: &doido_controller::axum::response::Response) -> String {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

async fn body_json(resp: doido_controller::axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn session_persists_and_flash_lives_one_request() {
    let app = app();

    // 1) bump → n becomes 1, session + flash cookies set on the response.
    let resp = app
        .clone()
        .oneshot(Request::get("/bump").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cookies = cookies_from(&resp);
    assert!(cookies.contains("_doido_session="), "session cookie set");
    assert!(cookies.contains("_doido_flash="), "flash cookie set");
    assert_eq!(body_json(resp).await["n"], 1);

    // 2) read carrying the cookies → session counter survived, flash readable.
    let resp = app
        .clone()
        .oneshot(
            Request::get("/read")
                .header(header::COOKIE, &cookies)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let next_cookies = cookies_from(&resp);
    let v = body_json(resp).await;
    assert_eq!(v["n"], 1, "session persisted across requests");
    assert_eq!(v["notice"], "bumped", "flash readable on the next request");

    // 3) read again carrying request 2's cookies → flash was swept (gone).
    let resp = app
        .oneshot(
            Request::get("/read")
                .header(header::COOKIE, &next_cookies)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert_eq!(v["n"], 1, "session still there");
    assert_eq!(v["notice"], "none", "flash lived exactly one request");
}
