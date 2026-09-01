//! Additional `Context` coverage for methods/branches not exercised by
//! `context_api_test`/`context_extended_test`/`ctx_session_test`/`render_test`.
//!
//! Kept in its own test binary and deliberately does NOT install the global view
//! engine, so `render` hits its uninitialised-engine error branch (→ 500).

use doido_controller::axum::body::Body;
use doido_controller::respond::Format;
use doido_controller::Context;
use http::{header, Request, StatusCode};

fn parts(uri: &str) -> http::request::Parts {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap()
        .into_parts()
        .0
}

#[test]
fn render_without_engine_returns_500() {
    let ctx = Context::from_request_parts(parts("/"));
    let resp = ctx.render("no/such/template", serde_json::json!({ "a": 1 }));
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn header_reads_request_header() {
    let ctx = Context::from_request_parts({
        let mut p = parts("/");
        p.headers
            .insert("x-test", header::HeaderValue::from_static("yes"));
        p
    });
    assert_eq!(ctx.header("x-test").unwrap(), "yes");
    assert!(ctx.header("x-absent").is_none());
}

#[tokio::test]
async fn from_request_constructor_reads_form_body() {
    let ctx_parts = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    let mut ctx = Context::from_request(ctx_parts, Body::from("name=doido"));
    #[derive(serde::Deserialize)]
    struct Form {
        name: String,
    }
    let form: Form = ctx.form().await.unwrap();
    assert_eq!(form.name, "doido");
}

#[test]
fn negotiated_format_honours_accept_header() {
    let html = Context::from_request_parts({
        let mut p = parts("/posts");
        p.headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("text/html"),
        );
        p
    });
    assert_eq!(html.negotiated_format(), Format::Html);

    let any = Context::from_request_parts({
        let mut p = parts("/posts");
        p.headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/octet-stream"),
        );
        p
    });
    assert_eq!(any.negotiated_format(), Format::Any);

    // No extension, no Accept header at all → Any.
    assert_eq!(
        Context::from_request_parts(parts("/posts")).negotiated_format(),
        Format::Any
    );
}

#[test]
fn respond_to_uses_negotiated_format() {
    let ctx = Context::from_request_parts(parts("/posts.json"));
    // Just constructing it exercises `respond_to()` / `negotiated_format()`.
    let _responder = ctx.respond_to();
    assert!(ctx.wants_json());
}

#[test]
fn etag_matches_wildcard_and_list() {
    let star = Context::from_request_parts({
        let mut p = parts("/asset");
        p.headers
            .insert(header::IF_NONE_MATCH, header::HeaderValue::from_static("*"));
        p
    });
    assert!(star.etag_matches("\"anything\""));

    let list = Context::from_request_parts({
        let mut p = parts("/asset");
        p.headers.insert(
            header::IF_NONE_MATCH,
            header::HeaderValue::from_static("\"a\", \"b\""),
        );
        p
    });
    assert!(list.etag_matches("\"b\""));
    assert!(!list.etag_matches("\"c\""));
}

#[test]
fn fresh_when_last_modified_returns_304() {
    let lm = "Wed, 21 Oct 2026 07:28:00 GMT";
    let ctx = Context::from_request_parts({
        let mut p = parts("/asset");
        p.headers.insert(
            header::IF_MODIFIED_SINCE,
            header::HeaderValue::from_static("Wed, 21 Oct 2026 07:28:00 GMT"),
        );
        p
    });
    let not_modified = ctx.fresh_when(None, Some(lm)).unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        not_modified.headers().get(header::LAST_MODIFIED).unwrap(),
        lm
    );

    // A non-matching validator renders normally (None).
    let stale = Context::from_request_parts(parts("/asset"));
    assert!(stale.fresh_when(None, Some(lm)).is_none());
}
