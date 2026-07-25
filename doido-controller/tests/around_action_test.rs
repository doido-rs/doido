use axum::{body::Body, response::Response, routing::get, Router};
use doido_controller::{controller, Context};
use http::{Request, StatusCode};
use tower::ServiceExt;

// An around filter brackets the action: it can run code before and after, and
// it owns the resulting response (here it tags it with a header).
async fn wrap(ctx: &mut Context, run: impl AsyncFnOnce(&mut Context) -> Response) -> Response {
    let mut resp = run(ctx).await;
    resp.headers_mut().insert("x-wrapped", "1".parse().unwrap());
    resp
}

struct WrappedController;

#[controller]
impl WrappedController {
    #[around_action(wrap)]
    async fn show(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

#[tokio::test]
async fn around_action_wraps_the_response() {
    let app = Router::new().route("/", get(WrappedController::show));
    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("x-wrapped").unwrap(),
        "1",
        "the around filter post-processed the response"
    );
}
