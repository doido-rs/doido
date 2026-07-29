use doido_view::helpers::form::{
    form_end, form_tag, hidden_field, label, submit, text_area, text_field,
};

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

#[test]
fn form_end_hidden_field_and_label() {
    assert_eq!(form_end(), "</form>");
    assert_eq!(
        hidden_field("id", "<1>"),
        "<input type=\"hidden\" name=\"id\" value=\"&lt;1&gt;\">"
    );
    assert_eq!(
        label("title", "<Title>"),
        "<label for=\"title\">&lt;Title&gt;</label>"
    );
}

#[test]
fn delete_form_tunnels_method_and_escapes_csrf() {
    let f = form_tag("/posts/1", "delete", Some("<tok>"));
    assert!(f.contains("name=\"_method\" value=\"DELETE\""));
    assert!(f.contains("value=\"&lt;tok&gt;\""));
}
