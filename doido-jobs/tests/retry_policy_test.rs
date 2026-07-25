use doido_jobs::retry::{Decision, RetryPolicy};

#[derive(Debug)]
struct Transient;
impl std::fmt::Display for Transient {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "transient")
    }
}
impl std::error::Error for Transient {}

#[derive(Debug)]
struct Fatal;
impl std::fmt::Display for Fatal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "fatal")
    }
}
impl std::error::Error for Fatal {}

#[test]
fn retry_on_and_discard_on_by_type() {
    let policy = RetryPolicy::new()
        .retry_on::<Transient>()
        .discard_on::<Fatal>();

    assert_eq!(
        policy.decide(&doido_core::anyhow::Error::new(Transient)),
        Decision::Retry
    );
    assert_eq!(
        policy.decide(&doido_core::anyhow::Error::new(Fatal)),
        Decision::Discard
    );
    // unmatched errors retry by default
    assert_eq!(
        policy.decide(&doido_core::anyhow::anyhow!("other")),
        Decision::Retry
    );
}
