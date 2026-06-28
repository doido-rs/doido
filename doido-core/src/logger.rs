//! Centralized logging setup for the whole framework.
//!
//! [`init`] installs a global `tracing_subscriber` once at application boot.
//! Everything that emits `tracing` events then flows through it:
//!
//! - **HTTP requests/responses** — `doido-controller`'s middleware stack logs
//!   each request and response at `INFO` (via `tower_http`'s `TraceLayer`).
//! - **ORM queries** — `doido-model`'s connection enables sea-orm's SQL logging,
//!   which emits `tracing` events.
//! - **Jobs, mail, custom events** — the helpers in [`crate::trace`].
//!
//! The verbosity is controlled by the `RUST_LOG` environment variable
//! (standard `tracing_subscriber` `EnvFilter` syntax); when unset, [`init`] falls
//! back to [`DEFAULT_DIRECTIVES`].

use std::sync::Once;
use tracing_subscriber::{fmt, EnvFilter};

/// Default `EnvFilter` directives when `RUST_LOG` is unset.
///
/// `info` shows app logs and the HTTP request/response events (emitted by
/// `tower_http` at INFO). `sqlx::query=info` surfaces sea-orm's SQL statements
/// (logged by sqlx under that target) while `sqlx=warn` quiets the rest of the
/// connection-pool chatter; `hyper`/`tower` internals are quieted too.
pub const DEFAULT_DIRECTIVES: &str =
    "info,sqlx=warn,sqlx::query=info,hyper=warn,tower=warn";

static INIT: Once = Once::new();

/// Installs the global tracing subscriber using `RUST_LOG` (or
/// [`DEFAULT_DIRECTIVES`]). Idempotent and safe to call more than once.
pub fn init() {
    init_with(DEFAULT_DIRECTIVES);
}

/// Like [`init`] but uses `default_directives` when `RUST_LOG` is unset.
pub fn init_with(default_directives: &str) {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(default_directives));
        // `try_init` returns Err if a subscriber is already set (e.g. in tests);
        // ignore it — the `Once` already guards against repeated setup here.
        let _ = fmt().with_env_filter(filter).with_target(true).try_init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_directives_are_valid() {
        // `try_new` errors on a malformed filter; the defaults must always parse.
        assert!(EnvFilter::try_new(DEFAULT_DIRECTIVES).is_ok());
    }
}
