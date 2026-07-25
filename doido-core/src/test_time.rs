//! Test time control (Rails `travel_to` / `travel`).
//!
//! Rust has no global mutable clock, so tests thread a [`TestClock`] through the
//! code under test (read time via `clock.now()`), then move it with `travel`/
//! `travel_to`.

use chrono::{DateTime, Duration, Utc};
use std::sync::Mutex;

/// A clock whose "now" can be set and advanced.
pub struct TestClock {
    now: Mutex<DateTime<Utc>>,
}

impl TestClock {
    /// A clock frozen at `at`.
    pub fn new(at: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(at),
        }
    }

    /// The current (frozen) time.
    pub fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }

    /// Jump to an absolute time (Rails `travel_to`).
    pub fn travel_to(&self, at: DateTime<Utc>) {
        *self.now.lock().unwrap() = at;
    }

    /// Advance by a duration (Rails `travel`).
    pub fn travel(&self, by: Duration) {
        let mut now = self.now.lock().unwrap();
        *now += by;
    }
}
