use doido_mailer::layout::apply_layout;

#[test]
fn layout_wraps_body_at_yield() {
    assert_eq!(
        apply_layout("<html><body>{{ yield }}</body></html>", "Hi!"),
        "<html><body>Hi!</body></html>"
    );
}

#[test]
fn layout_without_marker_appends_body() {
    assert_eq!(apply_layout("<header/>", "Hi!"), "<header/>Hi!");
}
