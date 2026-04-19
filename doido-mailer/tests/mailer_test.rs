use doido_mailer::{mailer, Deliverer, LogDeliverer, Mail, TestDeliverer};
use std::sync::Arc;

#[mailer]
struct WelcomeMailer;

impl WelcomeMailer {
    fn welcome(user_email: &str) -> Mail {
        Mail::new()
            .to(user_email)
            .subject("Welcome!")
            .body_text("Welcome to the platform.")
            .body_html("<h1>Welcome!</h1>")
    }
}

#[tokio::test]
async fn test_mailer_deliver_now_via_test_deliverer() {
    let deliverer = TestDeliverer::new();
    let mail = WelcomeMailer::welcome("alice@example.com");
    deliverer.deliver(&mail).await.unwrap();

    let sent = deliverer.sent().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "alice@example.com");
    assert_eq!(sent[0].subject, "Welcome!");
}

#[tokio::test]
async fn test_mailer_log_deliverer() {
    let deliverer = LogDeliverer;
    let mail = WelcomeMailer::welcome("bob@example.com");
    deliverer.deliver(&mail).await.unwrap();
}

#[tokio::test]
async fn test_deliverer_as_arc_dyn() {
    let deliverer: Arc<dyn Deliverer> = Arc::new(TestDeliverer::new());
    let mail = Mail::new()
        .to("c@example.com")
        .subject("Test")
        .body_text("body");
    deliverer.deliver(&mail).await.unwrap();
}

#[tokio::test]
async fn test_multiple_mails_captured() {
    let deliverer = TestDeliverer::new();
    deliverer
        .deliver(&Mail::new().to("a@b.com").subject("First").body_text("1"))
        .await
        .unwrap();
    deliverer
        .deliver(&Mail::new().to("b@c.com").subject("Second").body_text("2"))
        .await
        .unwrap();
    let sent = deliverer.sent().await;
    assert_eq!(sent.len(), 2);
}

#[tokio::test]
async fn test_mail_deliver_now_method() {
    let deliverer = TestDeliverer::new();
    let sent_count = {
        let mail = WelcomeMailer::welcome("dave@example.com");
        mail.deliver_now(&deliverer).await.unwrap();
        deliverer.sent().await.len()
    };
    assert_eq!(sent_count, 1);
}
