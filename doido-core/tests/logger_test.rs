//! Installs the global subscriber, so kept in its own test binary to avoid
//! clashing with other tests that set up tracing.

use doido_core::logger::{REQUEST_TARGET, RESPONSE_TARGET};

#[test]
fn init_is_idempotent_and_emits() {
    // Calling more than once must not panic.
    doido_core::logger::init();
    doido_core::logger::init();

    // The subscriber is now installed; emitting an event must not panic either.
    doido_core::tracing::info!("logger smoke test");
}

#[test]
fn init_with_custom_directives() {
    doido_core::logger::init_with("warn");
    doido_core::tracing::warn!("init_with smoke");
}

#[test]
fn request_and_response_targets_are_stable() {
    assert_eq!(REQUEST_TARGET, "doido::request");
    assert_eq!(RESPONSE_TARGET, "doido::response");
}
