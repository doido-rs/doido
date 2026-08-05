//! API-mode route generation. This crate's `config/application.toml` sets
//! `api_only = true`, so `resources!` here must omit the HTML-form routes
//! (`new`/`edit`) while keeping every JSON-relevant action, including `destroy`.

// The `new`/`edit` handlers are deliberately never routed in API mode.
#![allow(dead_code)]

use doido_controller::axum::body::Body;
use http::{Request, StatusCode};
use tower::ServiceExt;

mod posts_controller {
    use doido_controller::axum::extract::Path;
    pub async fn index() -> &'static str {
        "index"
    }
    pub async fn new() -> &'static str {
        "new"
    }
    pub async fn create() -> &'static str {
        "create"
    }
    pub async fn show(Path(_id): Path<u64>) -> &'static str {
        "show"
    }
    pub async fn edit(Path(_id): Path<u64>) -> &'static str {
        "edit"
    }
    pub async fn update(Path(_id): Path<u64>) -> &'static str {
        "update"
    }
    pub async fn destroy(Path(_id): Path<u64>) -> &'static str {
        "destroy"
    }
}

fn app() -> doido_controller::axum::Router {
    doido_controller::routes! { resources!(posts, posts_controller) }
}

async fn status(method: &str, uri: &str) -> StatusCode {
    app()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn api_mode_keeps_json_actions() {
    // index / show / create / update / destroy all survive API mode; destroy in
    // particular is a valid JSON action, unlike the `new`/`edit` form routes.
    assert_eq!(status("GET", "/posts").await, StatusCode::OK);
    assert_eq!(status("GET", "/posts/1").await, StatusCode::OK);
    assert_eq!(status("POST", "/posts").await, StatusCode::OK);
    assert_eq!(status("PATCH", "/posts/1").await, StatusCode::OK);
    assert_eq!(status("DELETE", "/posts/1").await, StatusCode::OK);
}

#[tokio::test]
async fn api_mode_drops_new_form_route() {
    // No `/posts/new` route: the request falls through to `/posts/{id}` (show),
    // whose `Path<u64>` rejects "new" — so the `new` handler never runs.
    let s = status("GET", "/posts/new").await;
    assert!(
        matches!(
            s,
            StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
        ),
        "expected new form route to be absent, got {s}"
    );
}

#[tokio::test]
async fn api_mode_drops_edit_form_route() {
    // `/posts/{id}/edit` is not registered at all in API mode.
    assert_eq!(status("GET", "/posts/1/edit").await, StatusCode::NOT_FOUND);
}

mod users_controller {
    use doido_controller::axum::extract::Path;
    pub async fn index() -> &'static str {
        "users"
    }
    pub async fn new() -> &'static str {
        "new"
    }
    pub async fn create() -> &'static str {
        "create"
    }
    pub async fn show(Path(_id): Path<u64>) -> &'static str {
        "show"
    }
    pub async fn edit(Path(_id): Path<u64>) -> &'static str {
        "edit"
    }
    pub async fn update(Path(_id): Path<u64>) -> &'static str {
        "update"
    }
    pub async fn destroy(Path(_id): Path<u64>) -> &'static str {
        "destroy"
    }
}

#[tokio::test]
async fn api_mode_propagates_through_namespace() {
    let app = doido_controller::routes! {
        namespace!(api, {
            resources!(users, users_controller)
        })
    };
    // Nested index survives...
    let index = app
        .clone()
        .oneshot(Request::get("/api/users").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(index.status(), StatusCode::OK);
    // ...but the nested edit form route is dropped too.
    let edit = app
        .oneshot(
            Request::get("/api/users/1/edit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(edit.status(), StatusCode::NOT_FOUND);
}
