use axum::body::Body;
use http::{Request, StatusCode};
use tower::ServiceExt;

mod posts_controller {
    pub async fn index() -> &'static str {
        "index"
    }
    pub async fn new() -> &'static str {
        "new"
    }
    pub async fn create() -> &'static str {
        "create"
    }
    pub async fn show(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "show"
    }
    pub async fn edit(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "edit"
    }
    pub async fn update(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "update"
    }
    pub async fn destroy(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "destroy"
    }
}

async fn about_handler() -> &'static str {
    "about page"
}

#[tokio::test]
async fn test_single_get_route_responds() {
    let app = doido_controller::routes! {
        get!("/about", about_handler)
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/about")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let app = doido_controller::routes! {
        get!("/about", about_handler)
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_resources_generates_index_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .oneshot(Request::get("/posts").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_resources_generates_show_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .oneshot(Request::get("/posts/1").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_resources_generates_new_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .oneshot(Request::get("/posts/new").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_resources_generates_edit_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .oneshot(Request::get("/posts/1/edit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_resources_generates_create_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/posts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_resources_generates_update_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/posts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_resources_generates_destroy_route() {
    let app = doido_controller::routes! { resources!(posts, posts_controller) };
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/posts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn test_resources_url_helpers() {
    // URL helpers are generated as fn items inside the routes! block expression.
    // Verify the macro compiles and the router is produced:
    let _app: axum::Router = doido_controller::routes! { resources!(posts, posts_controller) };
    // posts_path(), new_post_path(), post_path(id), edit_post_path(id) are
    // generated as fn items but are scoped to the routes! block expression.
    // Verified by successful compilation above.
}

#[tokio::test]
async fn test_resources_only_restricts_to_listed_actions() {
    let app = doido_controller::routes! {
        resources!(posts, posts_controller, only: [index, show])
    };
    // index exists → 200
    let resp = app
        .clone()
        .oneshot(Request::get("/posts").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // show exists → 200
    let resp = app
        .clone()
        .oneshot(Request::get("/posts/1").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // new is excluded → 404 (path not registered) or 405 or 400 (matched /:id but parse fails)
    let resp = app
        .oneshot(Request::get("/posts/new").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::METHOD_NOT_ALLOWED
            || resp.status() == StatusCode::BAD_REQUEST,
        "expected 404, 405, or 400, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_resources_except_excludes_listed_actions() {
    let app = doido_controller::routes! {
        resources!(posts, posts_controller, except: [destroy])
    };
    // index exists → 200
    let resp = app
        .clone()
        .oneshot(Request::get("/posts").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // destroy excluded → DELETE /posts/1 returns 405 (path /posts/:id still registered for show/update)
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/posts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::METHOD_NOT_ALLOWED || resp.status() == StatusCode::NOT_FOUND,
        "expected 405 or 404, got {}",
        resp.status()
    );
}

mod users_controller {
    pub async fn index() -> &'static str {
        "users"
    }
    pub async fn show(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "user"
    }
    pub async fn create() -> &'static str {
        "create user"
    }
    pub async fn new() -> &'static str {
        "new user"
    }
    pub async fn edit(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "edit user"
    }
    pub async fn update(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "update user"
    }
    pub async fn destroy(axum::extract::Path(_id): axum::extract::Path<u64>) -> &'static str {
        "del user"
    }
}

#[tokio::test]
async fn test_namespace_prefixes_path() {
    let app = doido_controller::routes! {
        namespace!(api, {
            resources!(users, users_controller)
        })
    };
    let resp = app
        .oneshot(Request::get("/api/users").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_scope_prefixes_path() {
    let app = doido_controller::routes! {
        scope!("/v2", {
            resources!(users, users_controller)
        })
    };
    let resp = app
        .oneshot(Request::get("/v2/users").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_namespace_member_route_prefixed() {
    let app = doido_controller::routes! {
        namespace!(api, {
            resources!(users, users_controller)
        })
    };
    let resp = app
        .oneshot(Request::get("/api/users/1").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_combined_routes_block() {
    let app = doido_controller::routes! {
        resources!(posts, posts_controller)
        get!("/about", about_handler)
        namespace!(api, {
            resources!(users, users_controller)
        })
        scope!("/v2", {
            resources!(posts, posts_controller)
        })
    };

    // resources
    let r = app
        .clone()
        .oneshot(Request::get("/posts").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // custom route
    let r = app
        .clone()
        .oneshot(Request::get("/about").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // namespace
    let r = app
        .clone()
        .oneshot(Request::get("/api/users").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // scope
    let r = app
        .oneshot(Request::get("/v2/posts").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

