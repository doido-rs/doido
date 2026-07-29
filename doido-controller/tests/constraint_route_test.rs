//! `routes!` `constraints: { param: validator }` — a matched path param that
//! fails its validator 404s before the handler runs.

use axum::body::Body;
use doido_controller::constraints::numeric;
use http::{Request, StatusCode};
use tower::ServiceExt;

async fn show() -> &'static str {
    "shown"
}

#[tokio::test]
async fn numeric_constraint_passes_for_digits() {
    let app = doido_controller::routes! {
        get!("/posts/{id}", show, constraints: { id: numeric })
    };
    let resp = app
        .oneshot(Request::get("/posts/42").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn numeric_constraint_404s_for_non_digits() {
    let app = doido_controller::routes! {
        get!("/posts/{id}", show, constraints: { id: numeric })
    };
    let resp = app
        .oneshot(Request::get("/posts/abc").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn constraint_composes_with_named_route() {
    // `as:` and `constraints:` together, in either order.
    let app = doido_controller::routes! {
        get!("/users/{id}", show, as: user, constraints: { id: numeric })
    };
    let ok = app
        .clone()
        .oneshot(Request::get("/users/7").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let bad = app
        .oneshot(Request::get("/users/xyz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::NOT_FOUND);
}
