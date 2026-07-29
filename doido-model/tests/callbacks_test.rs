use doido_core::Result;
use doido_model::callbacks::{run_create, run_destroy, run_update, Callbacks};

#[derive(Default)]
struct Mock {
    fired: Vec<String>,
}
impl Mock {
    fn log(&mut self, s: &str) {
        self.fired.push(s.to_string());
    }
}
impl Callbacks for Mock {
    fn before_validation(&mut self) -> Result<()> {
        self.log("before_validation");
        Ok(())
    }
    fn before_save(&mut self) -> Result<()> {
        self.log("before_save");
        Ok(())
    }
    fn after_save(&mut self) -> Result<()> {
        self.log("after_save");
        Ok(())
    }
    fn before_create(&mut self) -> Result<()> {
        self.log("before_create");
        Ok(())
    }
    fn after_create(&mut self) -> Result<()> {
        self.log("after_create");
        Ok(())
    }
    fn before_update(&mut self) -> Result<()> {
        self.log("before_update");
        Ok(())
    }
    fn after_update(&mut self) -> Result<()> {
        self.log("after_update");
        Ok(())
    }
    fn before_destroy(&mut self) -> Result<()> {
        self.log("before_destroy");
        Ok(())
    }
    fn after_destroy(&mut self) -> Result<()> {
        self.log("after_destroy");
        Ok(())
    }
}

#[tokio::test]
async fn create_runs_callbacks_around_persist_in_order() {
    let mut m = Mock::default();
    run_create(&mut m, async |m: &mut Mock| {
        m.log("persist");
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        m.fired,
        [
            "before_validation",
            "before_save",
            "before_create",
            "persist",
            "after_create",
            "after_save"
        ]
    );
}

#[tokio::test]
async fn update_runs_update_callbacks_not_create() {
    let mut m = Mock::default();
    run_update(&mut m, async |m: &mut Mock| {
        m.log("persist");
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        m.fired,
        [
            "before_validation",
            "before_save",
            "before_update",
            "persist",
            "after_update",
            "after_save"
        ]
    );
}

#[tokio::test]
async fn destroy_runs_destroy_callbacks() {
    let mut m = Mock::default();
    run_destroy(&mut m, async |m: &mut Mock| {
        m.log("persist");
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(m.fired, ["before_destroy", "persist", "after_destroy"]);
}

#[tokio::test]
async fn a_failing_before_callback_aborts_persist() {
    struct Aborting {
        persisted: bool,
    }
    impl Callbacks for Aborting {
        fn before_save(&mut self) -> Result<()> {
            Err(doido_core::anyhow::anyhow!("halt"))
        }
    }
    let mut a = Aborting { persisted: false };
    let res = run_create(&mut a, async |a: &mut Aborting| {
        a.persisted = true;
        Ok(())
    })
    .await;
    assert!(res.is_err());
    assert!(
        !a.persisted,
        "persist is skipped when a before callback fails"
    );
}

#[tokio::test]
async fn before_validation_failure_skips_persist() {
    struct Aborting;
    impl Callbacks for Aborting {
        fn before_validation(&mut self) -> Result<()> {
            Err(doido_core::anyhow::anyhow!("invalid"))
        }
    }
    let mut m = Aborting;
    assert!(run_create(&mut m, async |_| Ok(())).await.is_err());
}

#[tokio::test]
async fn persist_failure_skips_after_callbacks() {
    struct Mock {
        fired: Vec<String>,
    }
    impl Mock {
        fn log(&mut self, s: &str) {
            self.fired.push(s.to_string());
        }
    }
    impl Callbacks for Mock {
        fn before_create(&mut self) -> Result<()> {
            self.log("before_create");
            Ok(())
        }
        fn after_create(&mut self) -> Result<()> {
            self.log("after_create");
            Ok(())
        }
    }
    let mut m = Mock { fired: vec![] };
    let err = run_create(&mut m, async |_| {
        Err(doido_core::anyhow::anyhow!("db error"))
    })
    .await;
    assert!(err.is_err());
    assert_eq!(m.fired, ["before_create"]);
}
