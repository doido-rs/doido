use doido_mailer::sendmail::SendmailDeliverer;
use doido_mailer::{Deliverer, Mail};

#[tokio::test]
async fn sendmail_pipes_the_message_to_the_binary() {
    // `cat` stands in for sendmail: it reads stdin and exits 0.
    let mail = Mail::new().to("b@y.com").subject("Hi").body_text("x");
    SendmailDeliverer::new("cat").deliver(&mail).await.unwrap();
}

#[tokio::test]
async fn sendmail_errors_when_the_binary_is_missing() {
    let mail = Mail::new().to("b@y.com").subject("Hi");
    assert!(SendmailDeliverer::new("/nonexistent/sendmail-xyz")
        .deliver(&mail)
        .await
        .is_err());
}

#[test]
fn sendmail_default_constructs() {
    let d = SendmailDeliverer::default();
    let _ = d;
}

#[tokio::test]
async fn sendmail_errors_on_nonzero_exit() {
    let mail = Mail::new().to("b@y.com").subject("Hi");
    let err = SendmailDeliverer::new("false")
        .deliver(&mail)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("exited"), "got: {err}");
}
