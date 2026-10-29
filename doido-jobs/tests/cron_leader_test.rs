use doido_jobs::config::{Backend, JobsConfig};
use doido_jobs::cron::{build_cron_leader, CronLeaderLock, NoopCronLeader};

#[tokio::test]
async fn noop_leader_always_acquires() {
    let lock = NoopCronLeader;
    assert!(lock.try_acquire("doido:cron:test:1").await.unwrap());
    assert!(lock.try_acquire("doido:cron:test:1").await.unwrap());
}

#[tokio::test]
async fn build_cron_leader_memory_backend() {
    let cfg = JobsConfig {
        backend: Backend::Memory,
        ..JobsConfig::default()
    };
    let leader = build_cron_leader(&cfg).await.unwrap();
    assert!(leader.try_acquire("doido:cron:job:1").await.unwrap());
}

#[cfg(feature = "jobs-db")]
#[tokio::test]
async fn build_cron_leader_db_sqlite_uses_noop_lock() {
    let _guard = doido_model::pool::test_lock();
    if doido_model::pool::try_pool().is_none() {
        let conn = doido_model::connect_with_url("sqlite::memory:")
            .await
            .unwrap();
        let _ = doido_model::pool::set_pool(conn);
    }
    let cfg = JobsConfig {
        backend: Backend::Db,
        ..JobsConfig::default()
    };
    let leader = build_cron_leader(&cfg).await.unwrap();
    assert!(leader.try_acquire("doido:cron:job:1").await.unwrap());
}
