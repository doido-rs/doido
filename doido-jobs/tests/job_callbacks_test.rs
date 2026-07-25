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
