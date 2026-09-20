//! Shared CLI boot policy (i18n + storage init behaviour for apps and the `doido` binary).

use std::path::Path;

/// How a boot step should react when initialization fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InitPolicy {
    /// Log a warning and continue (legacy `server` behaviour).
    Warn,
    /// Print to stderr and exit the process with code 1.
    #[default]
    FailFast,
}

/// Per-subsystem boot policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootPolicy {
    pub i18n: InitPolicy,
    pub storage: InitPolicy,
}

impl Default for BootPolicy {
    fn default() -> Self {
        // Match legacy `server` behaviour; apps like Fivia opt into [`InitPolicy::FailFast`]
        // via the [`doido::Doido`] builder.
        Self {
            i18n: InitPolicy::Warn,
            storage: InitPolicy::Warn,
        }
    }
}

/// Default directory for app locale YAML (`config/locales`).
pub const DEFAULT_LOCALES_DIR: &str = "config/locales";

/// Apply [`InitPolicy`] to an initialization error.
pub fn handle_init_error(policy: InitPolicy, context: &str, err: impl std::fmt::Display) {
    match policy {
        InitPolicy::Warn => {
            crate::tracing::warn!("{context}: {err}");
        }
        InitPolicy::FailFast => {
            eprintln!("{context}: {err}");
            std::process::exit(1);
        }
    }
}

/// Load app locales from `path` using [`BootPolicy::i18n`]. Idempotent.
pub fn install_i18n(path: &Path, policy: InitPolicy) {
    if let Err(e) = crate::i18n::init(path) {
        handle_init_error(policy, "failed to initialize i18n", e);
    }
}
