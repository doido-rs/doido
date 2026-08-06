use doido_controller::Helper;

#[doido_controller::helper]
pub struct PostsHelper;

impl PostsHelper {
    pub fn format_title(title: &str) -> String {
        title.trim().to_uppercase()
    }
}

#[test]
fn test_helper_name_is_snake_case_of_struct() {
    assert_eq!(PostsHelper::helper_name(), "posts_helper");
    assert_eq!(<PostsHelper as Helper>::helper_name(), "posts_helper");
}

#[test]
fn test_helper_methods_are_callable_from_controllers() {
    assert_eq!(PostsHelper::format_title("  hello  "), "HELLO");
}

struct PostsController;

#[doido_controller::controller]
impl PostsController {
    async fn index(ctx: doido_controller::Context) -> doido_controller::Response {
        let title = PostsHelper::format_title("world");
        ctx.json(serde_json::json!({ "title": title }))
    }
}

#[tokio::test]
async fn test_controller_uses_imported_helper() {
    use doido_controller::axum::body::Body;
    use http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = doido_controller::axum::Router::new().route(
        "/",
        doido_controller::axum::routing::get(PostsController::index),
    );

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["title"], "WORLD");
}
