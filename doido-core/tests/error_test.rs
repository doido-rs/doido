use doido_core::error::{AnyhowContext, Result};

// Define a typed error as any downstream crate would
#[derive(doido_core::thiserror::Error, Debug)]
enum FakeError {
    #[error("something went wrong: {0}")]
    Oops(String),
}

fn might_fail() -> std::result::Result<(), FakeError> {
    Err(FakeError::Oops("bad".into()))
}

fn propagate_via_question_mark() -> Result<()> {
    might_fail()?; // ? converts FakeError into anyhow::Error
    Ok(())
}

#[test]
fn test_thiserror_propagates_into_anyhow_result() {
    let result = propagate_via_question_mark();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("something went wrong: bad"));
}

#[test]
fn test_anyhow_context_adds_message() {
    let result: Result<()> = might_fail()
        .map_err(|e| doido_core::anyhow::anyhow!(e))
        .with_context(|| "extra context");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("extra context"), "got: {msg}");
}
