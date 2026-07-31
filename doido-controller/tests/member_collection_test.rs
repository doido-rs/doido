use doido_controller::axum::body::Body;
use doido_controller::axum::extract::Path;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod posts {
    use super::Path;
    pub async fn index() -> &'static str {
        "index"
    }
    pub async fn create() -> &'static str {
        "create"
    }
    pub async fn new() -> &'static str {
        "new"
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
    // member + collection actions
    pub async fn preview(Path(_id): Path<u64>) -> &'static str {
        "preview"
    }
    pub async fn search() -> &'static str {
        "search"
    }
}

async fn body_of(resp: doido_controller::axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn member_and_collection_routes_are_added() {
    let app = doido_controller::routes! {
        resources!(posts, posts, member: [preview], collection: [search]);
    };

    // collection: GET /posts/search
    let search = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/posts/search")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search.status(), StatusCode::OK);
    assert_eq!(body_of(search).await, "search");

    // member: GET /posts/1/preview
    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/posts/1/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview.status(), StatusCode::OK);
    assert_eq!(body_of(preview).await, "preview");

    // the standard show route still works
    let show = app
        .oneshot(
            Request::builder()
                .uri("/posts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(show.status(), StatusCode::OK);
    assert_eq!(body_of(show).await, "show");
}
