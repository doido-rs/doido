use doido_jobs::{BackoffStrategy, EngineConfig, JobPayload, JobQueue, MemoryQueue, WorkerEngine};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn config(queues: &[&str], concurrency: usize) -> EngineConfig {
    EngineConfig {
        queues: queues.iter().map(|s| s.to_string()).collect(),
        concurrency,
        poll_wait: Duration::from_millis(50),
        reclaim_interval: Duration::from_secs(60),
    }
}

#[test]
fn test_backoff_delays() {
    assert_eq!(BackoffStrategy::Exponential.delay(1, 5).as_secs(), 5);
    assert_eq!(BackoffStrategy::Exponential.delay(2, 5).as_secs(), 10);
    assert_eq!(BackoffStrategy::Exponential.delay(3, 5).as_secs(), 20);
    assert_eq!(BackoffStrategy::Exponential.delay(5, 5).as_secs(), 80);
    assert_eq!(BackoffStrategy::Linear.delay(3, 4).as_secs(), 12);
    assert_eq!(BackoffStrategy::None.delay(9, 5).as_secs(), 0);
}

#[tokio::test]
async fn test_engine_run_once_acks() {
    let queue: Arc<dyn JobQueue> = Arc::new(MemoryQueue::new());
    queue
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let engine = WorkerEngine::new(queue.clone(), config(&["default"], 1));
    let did = engine.run_once(&|_job| async { Ok(()) }).await.unwrap();
    assert!(did);
    // acked → nothing left
    let did2 = engine.run_once(&|_job| async { Ok(()) }).await.unwrap();
    assert!(!did2);
}

#[tokio::test]
async fn test_engine_timeout_is_a_failure() {
    let queue: Arc<dyn JobQueue> = Arc::new(MemoryQueue::new());
    // timeout = 0 forces the attempt to time out; max_retries 0 → straight to dead letter.
    let job = JobPayload::new("default", json!({}), 0).with_timeout(0);
    queue.enqueue(job).await.unwrap();
    let engine = WorkerEngine::new(queue.clone(), config(&["default"], 1));
    engine
        .run_once(&|_job| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(queue.dead_jobs("default").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_engine_processes_multiple_queues() {
    let queue: Arc<dyn JobQueue> = Arc::new(MemoryQueue::new());
    queue
        .enqueue(JobPayload::new("low", json!({}), 3))
        .await
        .unwrap();
    queue
        .enqueue(JobPayload::new("high", json!({}), 3))
        .await
        .unwrap();
    let engine = WorkerEngine::new(queue.clone(), config(&["high", "low"], 1));

    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let handler = move |_job: JobPayload| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    };
    engine.run_once(&handler).await.unwrap();
    engine.run_once(&handler).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_engine_run_drains_on_shutdown() {
    let queue: Arc<dyn JobQueue> = Arc::new(MemoryQueue::new());
    for _ in 0..5 {
        queue
            .enqueue(JobPayload::new("default", json!({}), 3))
            .await
            .unwrap();
    }
    let engine = WorkerEngine::new(queue.clone(), config(&["default"], 4));

    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let handler = move |_job: JobPayload| {
        let c = c.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    };

    // Let it run briefly, then signal shutdown; in-flight jobs must finish.
    let shutdown = async {
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    engine.run(handler, shutdown).await.unwrap();

    assert_eq!(count.load(Ordering::SeqCst), 5);
    assert!(queue
        .reserve(&["default"], Duration::from_millis(10))
        .await
        .unwrap()
        .is_none());
}
