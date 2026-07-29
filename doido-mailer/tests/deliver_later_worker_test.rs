//! End-to-end proof that `deliver_later` no longer no-ops: the enqueued mail is
//! picked up by the worker's registry dispatch and handed to the deliverer.

use doido_jobs::{EngineConfig, JobContext, JobQueue, JobRegistry, MemoryQueue, WorkerEngine};
use doido_mailer::{global, Mail, TestDeliverer};
use std::sync::Arc;

#[tokio::test]
async fn deliver_later_is_delivered_by_the_worker() {
    // Install a capturing deliverer as the global the mailer job uses.
    let test = TestDeliverer::new();
    global::set_deliverer(Arc::new(test.clone())).ok();

    let queue = Arc::new(MemoryQueue::new());
    Mail::new()
        .to("b@y.com")
        .subject("Later")
        .body_text("hi")
        .deliver_later(queue.as_ref())
        .await
        .unwrap();

    // Nothing delivered until the worker runs.
    assert!(test.sent().await.is_empty());

    let engine = WorkerEngine::with_context(
        Arc::clone(&queue) as Arc<dyn JobQueue>,
        EngineConfig {
            queues: vec!["mailers".to_string()],
            ..EngineConfig::default()
        },
        JobContext::new(),
    );
    let registry = JobRegistry::from_inventory();
    let handler = |job, ctx| {
        let registry = registry.clone();
        async move { registry.dispatch(job, ctx).await }
    };

    assert!(engine.run_once(&handler).await.unwrap(), "mailer job ran");

    let sent = test.sent().await;
    assert_eq!(sent.len(), 1, "the mail was delivered");
    assert_eq!(sent[0].to, ["b@y.com"]);
    assert_eq!(sent[0].subject, "Later");
    // Acked, not dead-lettered.
    assert!(queue.dead_jobs("mailers").await.unwrap().is_empty());
}
