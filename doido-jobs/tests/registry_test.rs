//! The job registry closes the worker-dispatch gap: a `#[job]` self-registers,
//! and a reserved payload is routed back to its function and actually runs.

use doido_jobs::{
    EngineConfig, JobContext, JobPayload, JobQueue, JobRegistry, MemoryQueue, WorkerEngine,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static RAN_TOTAL: AtomicU32 = AtomicU32::new(0);

/// A context-free job: adds its payload to a global counter so the test can
/// observe that the body actually executed (not just acked).
#[doido_jobs::job]
async fn add_to_total(n: u32) -> doido_core::Result<()> {
    RAN_TOTAL.fetch_add(n, Ordering::SeqCst);
    Ok(())
}

/// A context-carrying job (the generator-template shape: `&JobContext` first).
/// Reads a shared counter out of the context and bumps it.
#[doido_jobs::job]
async fn bump_ctx_counter(ctx: &JobContext, by: u32) -> doido_core::Result<()> {
    let counter = ctx
        .get::<AtomicU32>()
        .expect("counter registered in JobContext");
    counter.fetch_add(by, Ordering::SeqCst);
    Ok(())
}

/// Build a dispatching handler over the registry, exactly like `doido worker`.
fn dispatcher(
    registry: JobRegistry,
) -> impl Fn(
    JobPayload,
    Arc<JobContext>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = doido_core::Result<()>> + Send>> {
    move |job, ctx| {
        let registry = registry.clone();
        Box::pin(async move { registry.dispatch(job, ctx).await })
    }
}

#[tokio::test]
async fn dispatches_reserved_job_to_its_handler() {
    let queue = Arc::new(MemoryQueue::new());
    add_to_total_enqueue(queue.as_ref(), 7).await.unwrap();

    let engine = WorkerEngine::with_context(
        Arc::clone(&queue) as Arc<dyn JobQueue>,
        EngineConfig::default(),
        JobContext::new(),
    );
    let handler = dispatcher(JobRegistry::from_inventory());

    assert!(
        engine.run_once(&handler).await.unwrap(),
        "a job was processed"
    );
    assert_eq!(RAN_TOTAL.load(Ordering::SeqCst), 7, "job body ran");
    // Acked: nothing left ready, nothing dead-lettered.
    assert!(!engine.run_once(&handler).await.unwrap(), "queue drained");
    assert!(queue.dead_jobs("default").await.unwrap().is_empty());
}

#[tokio::test]
async fn job_reads_the_application_context() {
    let queue = Arc::new(MemoryQueue::new());
    bump_ctx_counter_enqueue(queue.as_ref(), 4).await.unwrap();

    let mut ctx = JobContext::new();
    ctx.insert(AtomicU32::new(0));
    let engine = WorkerEngine::with_context(
        Arc::clone(&queue) as Arc<dyn JobQueue>,
        EngineConfig::default(),
        ctx,
    );
    let handler = dispatcher(JobRegistry::from_inventory());

    assert!(engine.run_once(&handler).await.unwrap());
    let counter = engine.context().get::<AtomicU32>().unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn unknown_job_name_is_dead_lettered() {
    let queue = Arc::new(MemoryQueue::new());
    // A payload no `#[job]` registered a handler for, with no retries left.
    let job = JobPayload::new("default", serde_json::json!({}), 0).with_name("no_such_job");
    queue.enqueue(job).await.unwrap();

    let engine = WorkerEngine::with_context(
        Arc::clone(&queue) as Arc<dyn JobQueue>,
        EngineConfig::default(),
        JobContext::new(),
    );
    let handler = dispatcher(JobRegistry::from_inventory());

    // Processed (reserved + failed), then dead-lettered rather than silently acked.
    assert!(engine.run_once(&handler).await.unwrap());
    let dead = queue.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].job_name, "no_such_job");
}

#[test]
fn registry_lists_the_defined_jobs() {
    let names = JobRegistry::from_inventory().names();
    assert!(names.contains(&"add_to_total"));
    assert!(names.contains(&"bump_ctx_counter"));
}

#[tokio::test]
async fn manual_register_dispatches_handler() {
    use doido_jobs::queue::JobPayload;

    static HIT: AtomicU32 = AtomicU32::new(0);

    fn handler(_job: JobPayload, _ctx: Arc<JobContext>) -> doido_jobs::registry::HandlerFuture {
        Box::pin(async {
            HIT.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    let mut registry = JobRegistry::new();
    registry.register("manual_job", handler);
    registry
        .dispatch(
            JobPayload::new("default", serde_json::json!({}), 0).with_name("manual_job"),
            Arc::new(JobContext::new()),
        )
        .await
        .unwrap();
    assert_eq!(HIT.load(Ordering::SeqCst), 1);
}
