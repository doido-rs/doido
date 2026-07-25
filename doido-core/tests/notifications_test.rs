use doido_core::notifications::Notifications;
use std::sync::{Arc, Mutex};

#[test]
fn subscribers_receive_only_matching_events() {
    let hits = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut bus = Notifications::new();
    let h = hits.clone();
    bus.subscribe("sql.", move |name, _payload| {
        h.lock().unwrap().push(name.to_string())
    });

    bus.instrument("sql.query", "SELECT 1");
    bus.instrument("http.request", "/");
    bus.instrument("sql.commit", "");

    assert_eq!(*hits.lock().unwrap(), vec!["sql.query", "sql.commit"]);
}
