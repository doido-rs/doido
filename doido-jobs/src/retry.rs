//! Conditional retry/discard by error type (Rails `retry_on` / `discard_on`).
//!
//! A [`RetryPolicy`] maps error types to a [`Decision`]. When a job fails, the
//! worker asks the policy whether to retry (re-enqueue with backoff) or discard
//! (drop the job). The first matching rule wins; unmatched errors default to
//! [`Decision::Retry`].
//!
//! (Exponential backoff itself is applied by the worker/queue on `nack`.)

use doido_core::anyhow;

/// What to do with a failed job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Retry,
    Discard,
}

type Matcher = Box<dyn Fn(&anyhow::Error) -> Option<Decision> + Send + Sync>;

/// An ordered set of error-type rules.
#[derive(Default)]
pub struct RetryPolicy {
    matchers: Vec<Matcher>,
}

impl RetryPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retry when the error is (or wraps) `E` (Rails `retry_on E`).
    pub fn retry_on<E>(mut self) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.matchers.push(Box::new(|err| {
            err.downcast_ref::<E>().map(|_| Decision::Retry)
        }));
        self
    }

    /// Discard when the error is (or wraps) `E` (Rails `discard_on E`).
    pub fn discard_on<E>(mut self) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.matchers.push(Box::new(|err| {
            err.downcast_ref::<E>().map(|_| Decision::Discard)
        }));
        self
    }

    /// Decide what to do with `err`: the first matching rule, else `Retry`.
    pub fn decide(&self, err: &anyhow::Error) -> Decision {
        self.matchers
            .iter()
            .find_map(|m| m(err))
            .unwrap_or(Decision::Retry)
    }
}
