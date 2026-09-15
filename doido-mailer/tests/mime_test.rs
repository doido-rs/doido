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
fn from_header_keeps_display_name() {
    let mail = Mail::new()
        .from("Fivia App <app@fivia.com.br>")
        .to("user@gmail.com")
        .subject("Hi")
        .body_text("plain")
        .body_html("<p>html</p>");
    let msg = to_mime(&mail);

    // The MIME `From:` header preserves the full RFC 5322 display-name value —
    // only the SMTP `MAIL FROM` envelope (in smtp.rs) is reduced to the addr-spec.
    assert!(
        msg.contains("From: Fivia App <app@fivia.com.br>"),
        "From header keeps display name: {msg}"
    );
    assert!(msg.contains("multipart/alternative"));
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
fn subject_with_trailing_newline_breaks_mime_boundaries_fixed() {
    let mail = Mail::new()
        .from("app@example.com")
        .to("user@example.com")
        .subject("Hello\n")
        .body_html("<!DOCTYPE html><html><body><p>Hi</p></body></html>");

    let raw = to_mime(&mail);

    assert!(
        !raw.contains("Subject: Hello\n\r\n"),
        "subject must not contain bare LF inside the Subject header line"
    );

    let split = raw.find("\r\n\r\n").expect("CRLF CRLF separator");
    let headers = &raw[..split];
    assert!(headers.contains("Content-Type: text/html"));
    assert!(headers.contains("MIME-Version: 1.0"));
}

#[test]
fn subject_with_internal_newline_is_sanitized() {
    let mail = Mail::new()
        .to("user@example.com")
        .subject("Hello\r\nWorld")
        .body_html("<p>Hi</p>");

    let raw = to_mime(&mail);

    assert!(raw.contains("Subject: HelloWorld\r\n"));
    let split = raw.find("\r\n\r\n").expect("CRLF CRLF separator");
    let headers = &raw[..split];
    assert!(headers.contains("Content-Type: text/html"));
    assert!(headers.contains("MIME-Version: 1.0"));
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
