use doido_mailer::{Mail, TestDeliverer};

#[test]
fn test_mail_builder_defaults() {
    let m = Mail::new()
        .to("user@example.com")
        .subject("Hello")
        .body_text("Hi there");
    assert_eq!(m.to, "user@example.com");
    assert_eq!(m.subject, "Hello");
    assert!(m.body_html.is_none());
    assert_eq!(m.body_text, Some("Hi there".to_string()));
}

#[test]
fn test_mail_with_html_body() {
    let m = Mail::new()
        .from("sender@example.com")
        .to("user@example.com")
        .subject("Test")
        .body_html("<p>Hello</p>")
        .body_text("Hello");
    assert_eq!(m.from, Some("sender@example.com".to_string()));
    assert_eq!(m.body_html, Some("<p>Hello</p>".to_string()));
}

#[test]
fn test_mail_serializes_to_json() {
    let m = Mail::new().to("a@b.com").subject("S").body_text("B");
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("a@b.com"));
}

#[tokio::test]
async fn test_deliver_now_uses_deliverer() {
    let d = TestDeliverer::new();
    let m = Mail::new().to("x@y.com").subject("Hi").body_text("body");
    m.deliver_now(&d).await.unwrap();
    let sent = d.sent().await;
    assert_eq!(sent.len(), 1);
}
