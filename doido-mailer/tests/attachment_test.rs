use base64::{engine::general_purpose::STANDARD, Engine as _};
use doido_mailer::mime::to_mime;
use doido_mailer::Mail;

#[test]
fn attachments_produce_multipart_mixed_with_base64() {
    let mail = Mail::new()
        .to("b@y.com")
        .subject("Report")
        .body_text("see attached")
        .attach("report.txt", "text/plain", b"hello file".to_vec());

    let msg = to_mime(&mail);
    assert!(msg.contains("multipart/mixed"));
    assert!(msg.contains("Content-Disposition: attachment; filename=\"report.txt\""));
    assert!(msg.contains("Content-Transfer-Encoding: base64"));

    // The encoded attachment decodes back to the original bytes.
    let encoded = STANDARD.encode(b"hello file");
    assert!(msg.contains(&encoded), "base64 payload present");
}

#[test]
fn inline_attachments_use_inline_disposition() {
    let mail = Mail::new()
        .to("b@y.com")
        .subject("Hi")
        .body_html("<img src=cid:logo>")
        .attach_inline("logo.png", "image/png", vec![1, 2, 3]);
    let msg = to_mime(&mail);
    assert!(msg.contains("Content-Disposition: inline; filename=\"logo.png\""));
}
