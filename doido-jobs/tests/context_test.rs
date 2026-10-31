use doido_jobs::{job_context_with_queue, JobContext, JobPayload, JobQueue, MemoryQueue};
use serde_json::json;
use std::sync::Arc;

struct AppConfig {
    name: String,
}

#[test]
fn context_carries_typed_app_state() {
    let mut ctx = JobContext::new();
    ctx.insert(AppConfig {
        name: "doido".into(),
    });
    ctx.insert(42u32);

    assert_eq!(ctx.get::<AppConfig>().unwrap().name, "doido");
    assert_eq!(*ctx.get::<u32>().unwrap(), 42);
    assert!(ctx.get::<String>().is_none(), "unregistered type is None");
}

#[tokio::test]
async fn context_enqueue_jobs_uses_backend_batch() {
    let queue: Arc<dyn JobQueue> = Arc::new(MemoryQueue::new());
    let ctx = job_context_with_queue(Arc::clone(&queue));
    let jobs = vec![
        JobPayload::new("default", json!(1), 3),
        JobPayload::new("default", json!(2), 3),
    ];
    let ids = ctx.enqueue_jobs(jobs).await.unwrap();
    assert_eq!(ids.len(), 2);
    let page = queue
        .as_ref()
        .list_jobs("default", doido_jobs::JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert_eq!(page.jobs.len(), 2);
}

#[cfg(feature = "jobs-db")]
#[test]
fn context_db_reads_global_pool() {
    use doido_model::pool;

    let _lock = pool::test_lock();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if pool::try_pool().is_none() {
            let conn = doido_model::connect_with_url("sqlite::memory:")
                .await
                .unwrap();
            let _ = pool::set_pool(conn);
        }
        let ctx = JobContext::new();
        ctx.db().ping().await.unwrap();
    });
}
