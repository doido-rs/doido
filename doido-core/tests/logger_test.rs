//! Installs the global subscriber, so kept in its own test binary to avoid
//! clashing with other tests that set up tracing.

#[test]
fn init_is_idempotent_and_emits() {
    // Calling more than once must not panic.
    doido_core::logger::init();
    doido_core::logger::init();

    // The subscriber is now installed; emitting an event must not panic either.
    doido_core::tracing::info!("logger smoke test");
}
