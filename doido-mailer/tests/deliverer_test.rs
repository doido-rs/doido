use doido_mailer::{Deliverer, LogDeliverer, Mail, TestDeliverer};
use std::sync::Arc;

#[tokio::test]
async fn test_log_deliverer_succeeds() {
    let d = LogDeliverer;
    let mail = Mail::new().to("a@b.com").subject("Hi").body_text("Hello");
    d.deliver(&mail).await.unwrap();
}

#[tokio::test]
async fn test_test_deliverer_captures_mail() {
    let d = TestDeliverer::new();
    let mail = Mail::new().to("a@b.com").subject("Welcome").body_text("Hi");
    d.deliver(&mail).await.unwrap();
    let sent = d.sent().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "a@b.com");
    assert_eq!(sent[0].subject, "Welcome");
}

#[tokio::test]
async fn test_deliverer_is_object_safe() {
    let d: Arc<dyn Deliverer> = Arc::new(LogDeliverer);
    let mail = Mail::new().to("a@b.com").subject("Test").body_text("body");
    d.deliver(&mail).await.unwrap();
}
