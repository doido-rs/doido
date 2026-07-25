use doido_controller::respond::Format;
use doido_controller::Context;
use http::StatusCode;
use serde_json::json;

fn ctx_with_accept(accept: &str) -> Context {
    let parts = http::Request::builder()
        .uri("/posts")
        .header("accept", accept)
        .body(())
        .unwrap()
        .into_parts()
        .0;
    Context::from_request_parts(parts)
}

#[test]
fn negotiates_json_and_serves_the_json_branch() {
    let ctx = ctx_with_accept("application/json");
    assert_eq!(ctx.negotiated_format(), Format::Json);

    let resp = ctx
        .respond_to()
        .html(|| ctx.status(200))
        .json(|| ctx.json(json!({"ok": true})))
        .finish();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );
}

#[test]
fn negotiates_html_and_serves_the_html_branch() {
    let ctx = ctx_with_accept("text/html");
    let resp = ctx
        .respond_to()
        .html(|| ctx.status(201))
        .json(|| ctx.json(json!({})))
        .finish();
    // The html branch returns 201 with an empty body.
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[test]
fn requested_format_without_a_handler_is_406() {
    let ctx = ctx_with_accept("application/json");
    let resp = ctx.respond_to().html(|| ctx.status(200)).finish();
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
}

#[test]
fn json_path_extension_wins_over_accept() {
    let parts = http::Request::builder()
        .uri("/posts.json")
        .header("accept", "text/html")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    let ctx = Context::from_request_parts(parts);
    assert_eq!(ctx.negotiated_format(), Format::Json);
}
