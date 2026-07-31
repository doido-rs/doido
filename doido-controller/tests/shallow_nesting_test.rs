use doido_controller::axum::body::Body;
use doido_controller::axum::extract::Path;
use http::{Request, StatusCode};
use tower::ServiceExt;

mod comments {
    use super::Path;
    // Collection actions are nested (carry the parent id).
    pub async fn index(Path(_post_id): Path<u64>) -> &'static str {
        "index"
    }
    pub async fn new(Path(_post_id): Path<u64>) -> &'static str {
        "new"
    }
    pub async fn create(Path(_post_id): Path<u64>) -> &'static str {
        "create"
    }
    // Member actions are shallow (only the child id).
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

async fn status(app: &doido_controller::axum::Router, method: &str, uri: &str) -> StatusCode {
    app.clone()
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
async fn shallow_resources_nest_collection_but_not_member() {
    let app = doido_controller::routes! {
        shallow_resources!(posts, comments, comments);
    };

    // Collection routes are nested under the parent.
    assert_eq!(
        status(&app, "GET", "/posts/1/comments").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "POST", "/posts/1/comments").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "GET", "/posts/1/comments/new").await,
        StatusCode::OK
    );

    // Member routes drop the parent (shallow).
    assert_eq!(status(&app, "GET", "/comments/9").await, StatusCode::OK);
    assert_eq!(
        status(&app, "GET", "/comments/9/edit").await,
        StatusCode::OK
    );
    assert_eq!(status(&app, "PATCH", "/comments/9").await, StatusCode::OK);
    assert_eq!(status(&app, "DELETE", "/comments/9").await, StatusCode::OK);

    // The non-shallow nested member path does NOT exist.
    assert_eq!(
        status(&app, "GET", "/posts/1/comments/9").await,
        StatusCode::NOT_FOUND
    );
}
