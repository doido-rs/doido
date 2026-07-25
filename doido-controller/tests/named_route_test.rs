use axum::body::Body;
use http::{Request, StatusCode};
use tower::ServiceExt;

async fn handler() -> &'static str {
    "ok"
}

#[tokio::test]
async fn named_custom_routes_parse_and_serve() {
    // `as: name` gives a custom route a `{name}_path()` helper, matching what
    // `resources!` already emits.
    let app = doido_controller::routes! {
        get!("/login", handler, as: login);
        post!("/logout", handler, as: logout);
    };

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);

    let logout = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::OK);
}
