use doido_controller::axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod profile {
    // Singular resource actions take no id.
    pub async fn show() -> &'static str {
        "show"
    }
    pub async fn new() -> &'static str {
        "new"
    }
    pub async fn create() -> &'static str {
        "create"
    }
    pub async fn edit() -> &'static str {
        "edit"
    }
    pub async fn update() -> &'static str {
        "update"
    }
    pub async fn destroy() -> &'static str {
        "destroy"
    }
}

async fn call(
    app: &doido_controller::axum::Router,
    method: &str,
    uri: &str,
) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn singular_resource_has_no_id_and_no_index() {
    let app = doido_controller::routes! {
        resource!(profile, profile);
    };

    assert_eq!(
        call(&app, "GET", "/profile").await,
        (StatusCode::OK, "show".into())
    );
    assert_eq!(
        call(&app, "GET", "/profile/new").await,
        (StatusCode::OK, "new".into())
    );
    assert_eq!(
        call(&app, "POST", "/profile").await,
        (StatusCode::OK, "create".into())
    );
    assert_eq!(
        call(&app, "GET", "/profile/edit").await,
        (StatusCode::OK, "edit".into())
    );
    assert_eq!(
        call(&app, "PATCH", "/profile").await,
        (StatusCode::OK, "update".into())
    );
    assert_eq!(
        call(&app, "DELETE", "/profile").await,
        (StatusCode::OK, "destroy".into())
    );
}
