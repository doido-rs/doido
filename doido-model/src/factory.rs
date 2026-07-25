//! Test factories (FactoryBot-style): build test records with unique sequences.
//!
//! A type implements [`Factory::build`] to produce a default instance — usually
//! using [`sequence`] for unique fields — and gets [`Factory::build_list`] for
//! free. Pair with [`crate::testing::TestDb`] to persist them.

use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// A process-global, monotonically increasing counter for unique test values
/// (FactoryBot `sequence`), e.g. `format!("user{}@example.com", sequence())`.
pub fn sequence() -> u64 {
    SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

/// Builds test instances of a type.
pub trait Factory: Sized {
    /// Build one instance (typically using [`sequence`] for unique fields).
    fn build() -> Self;

    /// Build `n` instances.
    fn build_list(n: usize) -> Vec<Self> {
        (0..n).map(|_| Self::build()).collect()
    }
}
