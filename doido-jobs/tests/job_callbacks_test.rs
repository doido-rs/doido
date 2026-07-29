use doido_core::Result;
use doido_jobs::callbacks::{run_perform, JobCallbacks};

#[derive(Default)]
struct Mock {
    log: Vec<String>,
    failed: bool,
}
impl JobCallbacks for Mock {
    fn before_perform(&mut self) -> Result<()> {
        self.log.push("before".into());
        Ok(())
    }
    fn after_perform(&mut self) -> Result<()> {
        self.log.push("after".into());
        Ok(())
    }
    fn on_failure(&mut self, _e: &doido_core::anyhow::Error) {
        self.failed = true;
    }
}

#[tokio::test]
async fn success_runs_before_and_after() {
    let mut job = Mock::default();
    run_perform(&mut job, async |j: &mut Mock| {
        j.log.push("perform".into());
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(job.log, ["before", "perform", "after"]);
    assert!(!job.failed);
}

#[tokio::test]
async fn failure_runs_on_failure_not_after() {
    let mut job = Mock::default();
    let res = run_perform(&mut job, async |_j: &mut Mock| {
        Err::<(), doido_core::anyhow::Error>(doido_core::anyhow::anyhow!("boom"))
    })
    .await;
    assert!(res.is_err());
    assert!(job.failed, "on_failure fired");
    assert_eq!(job.log, ["before"], "after did not run");
}

#[tokio::test]
async fn before_perform_failure_skips_work() {
    struct Aborting;
    impl JobCallbacks for Aborting {
        fn before_perform(&mut self) -> Result<()> {
            Err(doido_core::anyhow::anyhow!("halt"))
        }
    }
    let mut job = Aborting;
    assert!(run_perform(&mut job, async |_| Ok(())).await.is_err());
}

#[tokio::test]
async fn after_perform_failure_propagates() {
    struct AfterFail;
    impl JobCallbacks for AfterFail {
        fn after_perform(&mut self) -> Result<()> {
            Err(doido_core::anyhow::anyhow!("cleanup failed"))
        }
    }
    let mut job = AfterFail;
    assert!(run_perform(&mut job, async |_| Ok(())).await.is_err());
}

#[tokio::test]
async fn default_callback_hooks_are_noops() {
    struct Defaults;
    impl JobCallbacks for Defaults {}
    run_perform(&mut Defaults, async |_| Ok(())).await.unwrap();
}

#[tokio::test]
async fn default_on_failure_is_noop() {
    struct Defaults;
    impl JobCallbacks for Defaults {}
    assert!(run_perform(&mut Defaults, async |_| {
        Err(doido_core::anyhow::anyhow!("fail"))
    })
    .await
    .is_err());
}
