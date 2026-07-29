use doido_jobs::JobContext;

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
