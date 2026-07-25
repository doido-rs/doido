use axum::{body::Body, response::Response, routing::get, Router};
use doido_controller::{controller, Context};
use http::{Request, StatusCode};
use tower::ServiceExt;

async fn require_auth(ctx: &mut Context) -> Result<(), Response> {
    if ctx.header("x-auth").is_some() {
        Ok(())
    } else {
        Err(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap())
    }
}

struct ExceptController;

#[controller]
impl ExceptController {
    // Auth runs for every action *except* `public`.
    #[before_action(require_auth, except = [public])]
    async fn public(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }

    #[before_action(require_auth, except = [public])]
    async fn private(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

struct SkipController;

#[controller]
impl SkipController {
    // The before_action is cancelled by skip_before_action on this action.
    #[before_action(require_auth)]
    #[skip_before_action(require_auth)]
    async fn open(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

async fn status_of(handler: Router, uri: &str, auth: bool) -> StatusCode {
    let mut b = Request::builder().uri(uri);
    if auth {
        b = b.header("x-auth", "1");
    }
    handler
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn except_skips_the_filter_for_listed_action() {
    let app = Router::new()
        .route("/public", get(ExceptController::public))
        .route("/private", get(ExceptController::private));

    assert_eq!(
        status_of(app.clone(), "/public", false).await,
        StatusCode::OK,
        "filter skipped for `public`"
    );
    assert_eq!(
        status_of(app.clone(), "/private", false).await,
        StatusCode::UNAUTHORIZED,
        "filter runs for `private`"
    );
    assert_eq!(
        status_of(app, "/private", true).await,
        StatusCode::OK,
        "filter passes with auth"
    );
}

#[tokio::test]
async fn skip_before_action_cancels_the_filter() {
    let app = Router::new().route("/open", get(SkipController::open));
    assert_eq!(
        status_of(app, "/open", false).await,
        StatusCode::OK,
        "skip_before_action cancels require_auth"
    );
}
