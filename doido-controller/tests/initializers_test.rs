use doido_controller::initializers::Initializers;
use std::sync::{Arc, Mutex};

#[test]
fn initializers_run_in_registration_order() {
    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let mut inits = Initializers::new();
    let a = order.clone();
    inits.register("first", move || {
        a.lock().unwrap().push("first");
        Ok(())
    });
    let b = order.clone();
    inits.register("second", move || {
        b.lock().unwrap().push("second");
        Ok(())
    });

    assert_eq!(inits.names(), vec!["first", "second"]);
    inits.run_all().unwrap();
    assert_eq!(*order.lock().unwrap(), vec!["first", "second"]);
}

#[test]
fn a_failing_initializer_stops_the_sequence() {
    let mut inits = Initializers::new();
    inits.register("boom", || Err(doido_core::anyhow::anyhow!("nope")));
    inits.register("never", || panic!("should not run"));
    assert!(inits.run_all().is_err());
}
