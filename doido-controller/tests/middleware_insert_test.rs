use axum::{
    extract::Request, middleware::from_fn, middleware::Next, response::Response, routing::get,
    Router,
};
use doido_controller::MiddlewareStack;
use http::StatusCode;
use tower::ServiceExt;

async fn add_inner(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert("x-inner", "1".parse().unwrap());
    resp
}

async fn add_outer(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert("x-outer", "1".parse().unwrap());
    resp
}

#[tokio::test]
async fn insert_before_and_after_both_fire() {
    let stack = MiddlewareStack::new()
        .insert_before(|router| router.layer(from_fn(add_inner)))
        .insert_after(|router| router.layer(from_fn(add_outer)));
    let app = stack.apply(Router::new().route("/", get(|| async { "ok" })));

    let req = http::Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("x-inner").unwrap(),
        "1",
        "an inserted 'before' layer runs"
    );
    assert_eq!(
        resp.headers().get("x-outer").unwrap(),
        "1",
        "an inserted 'after' layer runs"
    );
}
