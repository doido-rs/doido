use doido_mailer::interceptors::InterceptingDeliverer;
use doido_mailer::{Mail, TestDeliverer};
use std::sync::atomic::{AtomicUsize, Ordering};

static OBSERVED: AtomicUsize = AtomicUsize::new(0);

fn redirect_to_sink(mail: &mut Mail) {
    mail.to = vec!["sink@test".to_string()];
}
fn count_observed(_mail: &Mail) {
    OBSERVED.fetch_add(1, Ordering::SeqCst);
}

#[tokio::test]
async fn interceptors_mutate_and_observers_see_delivery() {
    let test = TestDeliverer::new();
    let deliverer = InterceptingDeliverer::new(test.clone())
        .intercept(redirect_to_sink)
        .observe(count_observed);

    Mail::new()
        .to("real@example.com")
        .subject("Hi")
        .body_text("x")
        .deliver_now(&deliverer)
        .await
        .unwrap();

    let sent = test.sent().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].to,
        ["sink@test"],
        "interceptor rewrote the recipient"
    );
    assert_eq!(OBSERVED.load(Ordering::SeqCst), 1, "observer fired");
}
