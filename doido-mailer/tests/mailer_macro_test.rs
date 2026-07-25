//! Tests the `#[mailer]` macro expansion: it wires a mailer struct into the
//! framework by deriving the snake_case mailer name used for template
//! resolution (`mailers/<mailer_name>/<action>`, per docs/08-mailer.md), while
//! leaving the struct's own action methods untouched.

use doido_mailer::{mailer, Mail, TestDeliverer};

#[mailer]
struct UserMailer;

impl UserMailer {
    fn welcome(email: &str) -> Mail {
        Mail::new()
            .to(email)
            .subject("Welcome to Doido!")
            .body_text("hi")
    }
}

#[test]
fn mailer_name_is_snake_cased_from_struct() {
    assert_eq!(UserMailer::mailer_name(), "user_mailer");
}

#[test]
fn template_key_follows_the_convention() {
    assert_eq!(
        UserMailer::template_key("welcome"),
        "mailers/user_mailer/welcome"
    );
}

#[tokio::test]
async fn mailer_action_composes_and_delivers() {
    let deliverer = TestDeliverer::new();
    let mail = UserMailer::welcome("alice@example.com");
    mail.deliver_now(&deliverer).await.unwrap();

    let sent = deliverer.sent().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "alice@example.com");
    assert_eq!(sent[0].subject, "Welcome to Doido!");
}
