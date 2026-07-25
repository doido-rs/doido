use doido_mailer::mime::to_mime;
use doido_mailer::Mail;

#[test]
fn both_bodies_produce_multipart_alternative() {
    let mail = Mail::new()
        .to("b@y.com")
        .subject("Hi")
        .body_text("plain body")
        .body_html("<b>rich body</b>");
    let msg = to_mime(&mail);

    assert!(msg.contains("multipart/alternative"));
    assert!(msg.contains("boundary=\""));
    assert!(msg.contains("Content-Type: text/plain"));
    assert!(msg.contains("Content-Type: text/html"));
    assert!(msg.contains("plain body"));
    assert!(msg.contains("<b>rich body</b>"));
    assert!(msg.trim_end().ends_with("--"), "closes the multipart");
}

#[test]
fn single_body_produces_single_part() {
    let msg = to_mime(
        &Mail::new()
            .to("b@y.com")
            .subject("Hi")
            .body_html("<p>x</p>"),
    );
    assert!(!msg.contains("multipart"));
    assert!(msg.contains("Content-Type: text/html"));
}
