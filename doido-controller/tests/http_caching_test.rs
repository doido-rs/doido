use doido_controller::Context;
use http::StatusCode;

fn ctx(headers: &[(&str, &str)]) -> Context {
    let mut b = http::Request::builder().uri("/posts/1");
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    Context::from_request_parts(b.body(()).unwrap().into_parts().0)
}

#[test]
fn matching_etag_is_fresh_and_yields_304() {
    let ctx = ctx(&[("if-none-match", "\"abc\"")]);
    let resp = ctx
        .fresh_when(Some("\"abc\""), None)
        .expect("a matching ETag is fresh -> 304");
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(resp.headers().get("etag").unwrap(), "\"abc\"");
}

#[test]
fn mismatched_etag_is_not_fresh() {
    let ctx = ctx(&[("if-none-match", "\"old\"")]);
    assert!(ctx.fresh_when(Some("\"new\""), None).is_none());
}

#[test]
fn no_conditional_header_is_not_fresh() {
    let ctx = ctx(&[]);
    assert!(ctx.fresh_when(Some("\"abc\""), None).is_none());
}

#[test]
fn matching_if_modified_since_yields_304() {
    let lm = "Sun, 06 Nov 1994 08:49:37 GMT";
    let ctx = ctx(&[("if-modified-since", lm)]);
    let resp = ctx.fresh_when(None, Some(lm)).expect("not modified -> 304");
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[test]
fn star_if_none_match_is_always_fresh() {
    let ctx = ctx(&[("if-none-match", "*")]);
    assert!(ctx.fresh_when(Some("\"anything\""), None).is_some());
}
