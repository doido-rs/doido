use doido_view::helpers::hotwire::{stimulus_controller, turbo_frame, turbo_stream};

#[test]
fn hotwire_helpers() {
    assert_eq!(
        turbo_frame("messages", "<p>hi</p>"),
        "<turbo-frame id=\"messages\"><p>hi</p></turbo-frame>"
    );
    let stream = turbo_stream("append", "messages", "<li>x</li>");
    assert!(stream.contains("action=\"append\""));
    assert!(stream.contains("target=\"messages\""));
    assert!(stream.contains("<template><li>x</li></template>"));
    assert_eq!(stimulus_controller("hello"), "data-controller=\"hello\"");
}
