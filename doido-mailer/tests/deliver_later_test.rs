use doido_jobs::{JobQueue, MemoryQueue};
use doido_mailer::Mail;
use std::time::Duration;

#[tokio::test]
async fn deliver_later_enqueues_the_mail_on_the_mailers_queue() {
    let queue = MemoryQueue::new();
    let mail = Mail::new().to("b@y.com").subject("Later").body_text("hi");

    mail.deliver_later(&queue).await.unwrap();

    let reserved = queue
        .reserve(&["mailers"], Duration::from_millis(50))
        .await
        .unwrap()
        .expect("a mailer job was enqueued");
    let back: Mail = serde_json::from_value(reserved.job.payload).unwrap();
    assert_eq!(back.to, ["b@y.com"]);
    assert_eq!(back.subject, "Later");
}
