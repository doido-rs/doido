use doido_view::helpers::sanitize::{html_escape, sanitize, strip_tags};

#[test]
fn strip_tags_removes_markup_keeps_text() {
    assert_eq!(strip_tags("<b>hi</b> there"), "hi there");
    assert_eq!(
        strip_tags("<a href=\"/x\">link</a>"),
        "link",
        "attributes go with the tag"
    );
    assert_eq!(strip_tags("plain"), "plain");
}

#[test]
fn html_escape_encodes_specials() {
    assert_eq!(html_escape("<b>&\"'"), "&lt;b&gt;&amp;&quot;&#39;");
}

#[test]
fn sanitize_leaves_no_live_markup() {
    let out = sanitize("<script>alert(1)</script>Hello");
    assert!(!out.contains('<'), "no raw angle brackets remain: {out}");
    assert!(out.contains("Hello"));
}
