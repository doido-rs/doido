use doido_controller::axum::body::Body;
use doido_controller::axum::extract::Path;
use http::{Request, StatusCode};
use tower::ServiceExt;

mod comments {
    use super::Path;
    pub async fn index(Path(_post_id): Path<u64>) -> &'static str {
        "index"
    }
    pub async fn new(Path(_post_id): Path<u64>) -> &'static str {
        "new"
    }
    pub async fn create(Path(_post_id): Path<u64>) -> &'static str {
        "create"
    }
    pub async fn show(Path((_post_id, _id)): Path<(u64, u64)>) -> &'static str {
        "show"
    }
    pub async fn edit(Path((_post_id, _id)): Path<(u64, u64)>) -> &'static str {
        "edit"
    }
    pub async fn update(Path((_post_id, _id)): Path<(u64, u64)>) -> &'static str {
        "update"
    }
    pub async fn destroy(Path((_post_id, _id)): Path<(u64, u64)>) -> &'static str {
        "destroy"
    }
}

mod posts {
    pub async fn index() -> &'static str {
        "posts"
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
async fn nested_resource_full_member_paths() {
    let app = doido_controller::routes! {
        resource!(posts, posts, only: [index], {
            resource!(comments, comments);
        });
    };

    assert_eq!(status(&app, "GET", "/posts").await, StatusCode::OK);

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

    assert_eq!(
        status(&app, "GET", "/posts/1/comments/9").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "GET", "/posts/1/comments/9/edit").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "PATCH", "/posts/1/comments/9").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "DELETE", "/posts/1/comments/9").await,
        StatusCode::OK
    );

    // Shallow member path must not exist.
    assert_eq!(
        status(&app, "GET", "/comments/9").await,
        StatusCode::NOT_FOUND
    );
}
