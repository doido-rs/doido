use axum::body::Body;
use doido_controller::Context;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::Deserialize;
use tower::ServiceExt;

fn make_ctx(uri: &str) -> Context {
    let req = Request::builder().uri(uri).body(()).unwrap();
    let (parts, _) = req.into_parts();
    Context::from_request_parts(parts)
}

#[derive(Deserialize, Debug, PartialEq)]
struct SearchParams {
    q: String,
    page: Option<u32>,
}

#[tokio::test]
async fn test_ctx_params_deserializes_query_string() {
    let ctx = make_ctx("/search?q=hello&page=2");
    let p: SearchParams = ctx.params().unwrap();
    assert_eq!(p.q, "hello");
    assert_eq!(p.page, Some(2));
}

#[tokio::test]
async fn test_ctx_params_errors_on_invalid_input() {
    let ctx = make_ctx("/search?page=not_a_number");
    let result: doido_core::Result<SearchParams> = ctx.params();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ctx_json_returns_200_with_json_body() {
    let ctx = make_ctx("/");
    let resp = ctx.json(serde_json::json!({"ok": true}));
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap();
    assert!(ct.to_str().unwrap().contains("application/json"));
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["ok"], true);
}

#[tokio::test]
async fn test_ctx_redirect_to_returns_302_with_location() {
    let ctx = make_ctx("/");
    let resp = ctx.redirect_to("/dashboard");
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = resp.headers().get("location").unwrap();
    assert_eq!(loc.to_str().unwrap(), "/dashboard");
}

#[tokio::test]
async fn test_ctx_status_returns_custom_status_code() {
    let ctx = make_ctx("/");
    let resp = ctx.status(422);
    assert_eq!(resp.status().as_u16(), 422);
}

#[tokio::test]
async fn test_ctx_render_returns_ok_with_template_name() {
    let ctx = make_ctx("/");
    let resp = ctx.render("posts/index", serde_json::json!({}));
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(std::str::from_utf8(&body).unwrap().contains("posts/index"));
}

struct HelloController;

#[doido_controller::controller]
impl HelloController {
    async fn index(ctx: Context) -> doido_controller::Response {
        ctx.json(serde_json::json!({"message": "hello"}))
    }

    async fn show(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

#[tokio::test]
async fn test_controller_index_action_via_axum() {
    let app = axum::Router::new().route("/hello", axum::routing::get(HelloController::index));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/hello")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["message"], "hello");
}

#[tokio::test]
async fn test_controller_show_action_via_axum() {
    let app = axum::Router::new().route("/hello/{id}", axum::routing::get(HelloController::show));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/hello/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// Filter functions
async fn require_auth(ctx: &mut Context) -> Result<(), doido_controller::Response> {
    if ctx.header("x-auth-token").is_none() {
        return Err(ctx.status(401));
    }
    Ok(())
}

async fn set_locale(_ctx: &mut Context) -> Result<(), doido_controller::Response> {
    Ok(()) // always passes
}

struct SecureController;

#[doido_controller::controller]
impl SecureController {
    #[before_action(require_auth)]
    async fn secret(ctx: Context) -> doido_controller::Response {
        ctx.json(serde_json::json!({"secret": "data"}))
    }

    #[before_action(require_auth)]
    #[before_action(set_locale)]
    async fn double_filtered(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

#[tokio::test]
async fn test_before_action_halts_when_filter_returns_err() {
    let app = axum::Router::new().route("/secret", axum::routing::get(SecureController::secret));

    // No auth token — filter should return 401
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // With auth token — filter passes, action runs
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/secret")
                .header("x-auth-token", "valid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_multiple_before_actions_run_in_order() {
    let app = axum::Router::new().route(
        "/double",
        axum::routing::get(SecureController::double_filtered),
    );

    // Without auth — first filter halts
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/double")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // With auth — both filters pass, action runs
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/double")
                .header("x-auth-token", "valid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

async fn load_record(ctx: &mut Context) -> Result<(), doido_controller::Response> {
    // Halt with 404 when x-id header is "0"
    if ctx.header("x-id").map(|h| h.to_str().unwrap_or("")) == Some("0") {
        return Err(ctx.status(404));
    }
    Ok(())
}

struct ScopedController;

#[doido_controller::controller]
impl ScopedController {
    // load_record only fires for show and edit
    #[before_action(load_record, only = [show, edit])]
    async fn index(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }

    #[before_action(load_record, only = [show, edit])]
    async fn show(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }

    #[before_action(load_record, only = [show, edit])]
    async fn edit(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

#[tokio::test]
async fn test_before_action_only_fires_for_specified_actions() {
    let app = axum::Router::new()
        .route("/items", axum::routing::get(ScopedController::index))
        .route("/items/{id}", axum::routing::get(ScopedController::show))
        .route(
            "/items/{id}/edit",
            axum::routing::get(ScopedController::edit),
        );

    // index — filter NOT in `only` list → 200 even with x-id: 0
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/items")
                .header("x-id", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // show — filter fires, x-id: 0 → 404
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/items/1")
                .header("x-id", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // show — filter fires, x-id: 1 → 200
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/items/1")
                .header("x-id", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

thread_local! {
    static AFTER_FIRED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

async fn log_response(_ctx: &mut Context) {
    AFTER_FIRED.with(|f| f.set(true));
}

struct LoggedController;

#[doido_controller::controller]
impl LoggedController {
    #[after_action(log_response)]
    async fn index(ctx: Context) -> doido_controller::Response {
        ctx.status(200)
    }
}

#[tokio::test]
async fn test_after_action_fires_after_action_body() {
    AFTER_FIRED.with(|f| f.set(false));

    let app = axum::Router::new().route("/logged", axum::routing::get(LoggedController::index));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/logged")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(AFTER_FIRED.with(|f| f.get()), "after_action was not called");
}

async fn auth_guard(ctx: &mut Context) -> Result<(), doido_controller::Response> {
    if ctx.header("authorization").is_none() {
        return Err(ctx.status(401));
    }
    Ok(())
}

struct ArticlesController;

#[doido_controller::controller]
impl ArticlesController {
    #[before_action(auth_guard)]
    async fn index(ctx: Context) -> doido_controller::Response {
        ctx.json(serde_json::json!({"articles": []}))
    }

    async fn show(ctx: Context) -> doido_controller::Response {
        ctx.json(serde_json::json!({"id": 1}))
    }
}

#[tokio::test]
async fn test_full_stack_controller_with_filters_via_axum_router() {
    let app = axum::Router::new()
        .route("/articles", axum::routing::get(ArticlesController::index))
        .route(
            "/articles/{id}",
            axum::routing::get(ArticlesController::show),
        );

    // No auth — before_action halts with 401
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/articles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // With auth — action runs, returns JSON
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/articles")
                .header("authorization", "Bearer token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["articles"].is_array());

    // show has no filter — always 200
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/articles/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
