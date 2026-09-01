use doido_controller::axum::body::Body;
use doido_controller::Context;
use http::Request;

#[tokio::test]
async fn context_build_reads_path_params_and_form_body() {
    let req = Request::builder()
        .method("POST")
        .uri("/posts/42")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("title=Hello"))
        .unwrap();
    let mut ctx = Context::build(req).await;
    assert_eq!(ctx.param("id"), None); // no route match in bare request
    #[derive(serde::Deserialize)]
    struct Form {
        title: String,
    }
    let form: Form = ctx.form().await.unwrap();
    assert_eq!(form.title, "Hello");
}

#[tokio::test]
async fn context_body_json_deserializes_payload() {
    let req = Request::builder()
        .method("POST")
        .uri("/api")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"ok":true}"#))
        .unwrap();
    let mut ctx = Context::build(req).await;
    let value: serde_json::Value = ctx.body_json().await.unwrap();
    assert_eq!(value["ok"], true);
}

#[tokio::test]
async fn context_form_errors_when_body_already_consumed() {
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("a=1"))
        .unwrap();
    let mut ctx = Context::build(req).await;
    #[derive(serde::Deserialize)]
    struct Form {
        a: u8,
    }
    let _: Form = ctx.form().await.unwrap();
    let again: doido_core::Result<Form> = ctx.form().await;
    assert!(again.is_err());
}
