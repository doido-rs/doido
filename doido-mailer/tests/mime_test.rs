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
fn cc_appears_in_headers_but_bcc_does_not() {
    let mail = Mail::new()
        .to("to@x.com")
        .cc("cc1@x.com")
        .cc("cc2@x.com")
        .bcc("secret@x.com")
        .subject("Hi")
        .body_text("hello");
    let msg = to_mime(&mail);

    assert!(
        msg.contains("Cc: cc1@x.com, cc2@x.com"),
        "cc header present"
    );
    assert!(
        !msg.contains("secret@x.com"),
        "bcc never leaks into headers"
    );
}

#[test]
fn recipients_span_to_cc_and_bcc() {
    let mail = Mail::new().to("to@x.com").cc("cc@x.com").bcc("bcc@x.com");
    assert_eq!(mail.recipients(), vec!["to@x.com", "cc@x.com", "bcc@x.com"]);
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
