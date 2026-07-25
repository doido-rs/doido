use doido_view::helpers::form::{form_tag, submit, text_area, text_field};

#[test]
fn form_tag_tunnels_patch_and_includes_csrf() {
    let f = form_tag("/posts/1", "patch", Some("tok123"));
    assert!(f.contains("action=\"/posts/1\""));
    assert!(f.contains("method=\"post\""), "html method is post");
    assert!(
        f.contains("name=\"_method\" value=\"PATCH\""),
        "method override"
    );
    assert!(f.contains("name=\"authenticity_token\" value=\"tok123\""));
}

#[test]
fn get_and_post_forms_have_no_method_override() {
    assert!(!form_tag("/search", "get", None).contains("_method"));
    assert!(!form_tag("/posts", "post", None).contains("_method"));
}

#[test]
fn field_helpers_escape_values() {
    assert_eq!(
        text_field("title", "<x>"),
        "<input type=\"text\" name=\"title\" value=\"&lt;x&gt;\">"
    );
    assert!(text_area("body", "a & b").contains("a &amp; b"));
    assert!(submit("Save").contains("value=\"Save\""));
}
