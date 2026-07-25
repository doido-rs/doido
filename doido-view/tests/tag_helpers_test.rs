use doido_view::helpers::tag::{content_tag, tag};

#[test]
fn tag_builds_void_tags_with_attrs() {
    assert_eq!(tag("br", &[]), "<br>");
    assert_eq!(
        tag("input", &[("type", "text"), ("name", "q")]),
        "<input type=\"text\" name=\"q\">"
    );
}

#[test]
fn content_tag_wraps_escaped_content() {
    assert_eq!(
        content_tag("p", "hi", &[("class", "lead")]),
        "<p class=\"lead\">hi</p>"
    );
    assert!(content_tag("span", "a<b", &[]).contains("a&lt;b"));
}
