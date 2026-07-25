use doido_view::content_for::ContentFor;

#[test]
fn content_for_captures_and_yields_named_blocks() {
    let mut c = ContentFor::new();
    c.set("sidebar", "<nav>");
    c.set("sidebar", "</nav>"); // appends
    c.set("title", "Home");

    assert_eq!(c.get("sidebar"), "<nav></nav>");
    assert_eq!(c.get("title"), "Home");
    assert_eq!(c.get("missing"), "", "yield of an unset block is empty");
    assert!(c.has("sidebar"));
    assert!(!c.has("footer"));
}
