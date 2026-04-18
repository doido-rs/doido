use doido_view::response::ViewResponse;
use serde_json::json;

#[test]
fn test_view_response_defaults() {
    let r = ViewResponse::new("posts/index", json!({"x": 1}));
    assert_eq!(r.template, "posts/index");
    assert_eq!(r.status, 200);
    assert!(r.layout.is_none());
}

#[test]
fn test_view_response_status_builder() {
    let r = ViewResponse::new("posts/new", json!({})).status(422);
    assert_eq!(r.status, 422);
}

#[test]
fn test_view_response_layout_builder() {
    let r = ViewResponse::new("posts/index", json!({})).layout("admin");
    assert_eq!(r.layout, Some("admin".to_string()));
}

#[test]
fn test_view_response_no_layout_builder() {
    let r = ViewResponse::new("posts/index", json!({})).no_layout();
    assert_eq!(r.layout, Some("".to_string()));
}
