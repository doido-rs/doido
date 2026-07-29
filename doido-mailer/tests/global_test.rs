//! Process-global deliverer (`OnceLock` — one test binary).

use doido_mailer::{global, LogDeliverer, TestDeliverer};
use std::sync::Arc;

#[test]
fn deliverer_global_lifecycle() {
    let test = TestDeliverer::new();
    assert!(global::set_deliverer(Arc::new(test)).is_ok());

    let installed = global::deliverer();
    let _ = installed;

    let second = Arc::new(LogDeliverer);
    assert!(global::set_deliverer(second.clone()).is_err());

    let eager = global::init();
    let _ = eager;
}
