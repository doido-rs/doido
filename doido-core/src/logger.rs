//! Centralized logging setup for the whole framework.
//!
//! [`init`] installs a global `tracing_subscriber` once at application boot.
//! Everything that emits `tracing` events then flows through it:
//!
//! - **HTTP requests/responses** — `doido-controller`'s `logging::log_requests`
//!   middleware logs a `request` and a `response` line at `INFO`, correlated by
//!   a shared `request_id`.
//! - **ORM queries** — `doido-model`'s connection enables sea-orm's SQL logging,
//!   which emits `tracing` events under target `sqlx::query`.
//! - **Jobs, mail, custom events** — the helpers in [`crate::trace`].
//!
//! Applications configure the logger from the `logger` section of
//! `config/<env>.yml`, deserialized into [`LoggerConfig`] and applied via
//! [`init_with_config`]. That controls the verbosity (built from a log `level`
//! or an explicit `directives` filter), whether output is redirected to a
//! `file`, and whether sea-orm emits `sql` statement logs. Because sea-orm logs
//! through this same subscriber, redirecting to a file captures SQL too.
//!
//! The `RUST_LOG` environment variable, when set, overrides the configured
//! verbosity (env vars win over config, matching the rest of the framework).

use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Mutex, Once};
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::{fmt, EnvFilter};

/// `tracing` target for the "request received" event emitted by
/// `doido-controller`'s logging middleware.
pub const REQUEST_TARGET: &str = "doido::request";

/// `tracing` target for the "response sent" event emitted by `doido-controller`'s
/// logging middleware. [`LogFormat::JsonResponse`] isolates this target.
pub const RESPONSE_TARGET: &str = "doido::response";

/// Framework targets quieted below the application log level.
///
/// `sqlx::query=info` surfaces sea-orm's SQL statements (logged by sqlx under
/// that target) while `sqlx=warn` quiets the rest of the connection-pool
/// chatter; `hyper`/`tower` internals are quieted too. Appended after the app
/// level by [`directives_for_level`].
pub const NOISE_DIRECTIVES: &str = "sqlx=warn,sqlx::query=info,hyper=warn,tower=warn";

/// Default `EnvFilter` directives when `RUST_LOG` is unset.
///
/// `info` shows app logs and the HTTP request/response events (emitted by
/// `tower_http` at INFO), followed by the [`NOISE_DIRECTIVES`] noise reduction.
pub const DEFAULT_DIRECTIVES: &str = "info,sqlx=warn,sqlx::query=info,hyper=warn,tower=warn";

/// Builds `EnvFilter` directives for an application log `level` (e.g. `info`,
/// `debug`, `warn`), appending the framework [`NOISE_DIRECTIVES`] so SQL/HTTP
/// internals stay quiet regardless of the chosen level.
///
/// `directives_for_level("info")` is equivalent to [`DEFAULT_DIRECTIVES`].
pub fn directives_for_level(level: &str) -> String {
    format!("{level},{NOISE_DIRECTIVES}")
}

/// How log events are rendered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    /// Single-line, human-readable events (the default).
    #[default]
    Compact,
    /// Pretty, multi-line output with every field plus thread and source
    /// location — for inspecting all the structured data during development.
    Verbose,
    /// One JSON object per HTTP **response** event (status, latency and the
    /// correlation ids), suppressing everything else. Suited to access logs and
    /// latency metrics. Honours an explicit `directives`/`RUST_LOG` override if
    /// you need to widen it.
    JsonResponse,
}

/// Logging settings, deserialized from the `logger` section of
/// `config/<env>.yml`.
///
/// The application log `level` is combined with the framework's
/// [`NOISE_DIRECTIVES`] to build the `tracing` `EnvFilter`; set `directives` to
/// take full control of the filter string instead. When `file` is set, log
/// output is appended to that path (relative to the project root) instead of
/// stdout. `sql` toggles sea-orm's SQL statement logging. `format` selects the
/// output renderer ([`LogFormat`]).
#[derive(Debug, Clone, Deserialize)]
pub struct LoggerConfig {
    /// Base application log level: `trace`, `debug`, `info`, `warn`, or `error`.
    #[serde(default = "default_level")]
    pub level: String,
    /// Optional full `EnvFilter` directive string. When set it fully replaces
    /// the directives derived from `level` (and the built-in noise reduction),
    /// e.g. `info,my_app=debug,sqlx=warn`.
    #[serde(default)]
    pub directives: Option<String>,
    /// Optional path to redirect log output to (appended, ANSI colours off).
    /// Parent directories are created as needed. When unset, logs go to stdout.
    #[serde(default)]
    pub file: Option<String>,
    /// Whether sea-orm logs each SQL statement (target `sqlx::query`). Defaults
    /// to `true`; set `false` to silence query logging at the source.
    #[serde(default = "default_sql")]
    pub sql: bool,
    /// Output renderer: `compact` (default), `verbose`, or `json_response`.
    #[serde(default)]
    pub format: LogFormat,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_sql() -> bool {
    true
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            directives: None,
            file: None,
            sql: default_sql(),
            format: LogFormat::default(),
        }
    }
}

impl LoggerConfig {
    /// Resolves the `EnvFilter` directive string used as the fallback when
    /// `RUST_LOG` is unset: the explicit `directives` override if present;
    /// otherwise, for [`LogFormat::JsonResponse`], a filter isolating the
    /// [`RESPONSE_TARGET`]; otherwise the directives built from `level`.
    pub fn directives(&self) -> String {
        match (&self.directives, self.format) {
            (Some(directives), _) => directives.clone(),
            (None, LogFormat::JsonResponse) => format!("off,{RESPONSE_TARGET}=info"),
            (None, _) => directives_for_level(&self.level),
        }
    }
}

static INIT: Once = Once::new();

/// Installs the global tracing subscriber using `RUST_LOG` (or
/// [`DEFAULT_DIRECTIVES`]) and stdout output. Idempotent and safe to call more
/// than once.
pub fn init() {
    init_with_config(&LoggerConfig::default());
}

/// Like [`init`] but uses `default_directives` when `RUST_LOG` is unset.
pub fn init_with(default_directives: &str) {
    init_with_config(&LoggerConfig {
        directives: Some(default_directives.to_string()),
        ..LoggerConfig::default()
    });
}

/// Installs the global tracing subscriber from a [`LoggerConfig`]: verbosity
/// from `RUST_LOG` or the config's [`directives`](LoggerConfig::directives),
/// output to the config's `file` when set (otherwise stdout), and rendering per
/// the config's [`format`](LoggerConfig::format). Idempotent and safe to call
/// more than once; only the first call takes effect.
pub fn init_with_config(config: &LoggerConfig) {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(config.directives()));

        // Box the writer so the builder is one type whether output is a file or
        // stdout; only the formatter then varies per `format` below. A log file
        // gets no ANSI colour escapes; stdout keeps tty auto-detection.
        let (writer, to_file) = match open_log_file(config.file.as_deref()) {
            Some(file) => (BoxMakeWriter::new(Mutex::new(file)), true),
            None => (BoxMakeWriter::new(std::io::stdout), false),
        };
        let builder = fmt().with_env_filter(filter).with_writer(writer);
        let builder = if to_file {
            builder.with_ansi(false)
        } else {
            builder
        };

        // `try_init` returns Err if a subscriber is already set (e.g. in tests);
        // ignore it — the `Once` already guards against repeated setup here.
        match config.format {
            LogFormat::Compact => {
                let _ = builder.with_target(true).try_init();
            }
            LogFormat::Verbose => {
                let _ = builder
                    .pretty()
                    .with_target(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .try_init();
            }
            LogFormat::JsonResponse => {
                // Flatten the event fields (status, latency_ms, request_id, …)
                // to the top level of each JSON line.
                let _ = builder.json().flatten_event(true).try_init();
            }
        }
    });
}

/// Opens the configured log `file` for appending, creating parent directories
/// as needed. Returns `None` (logging stays on stdout) when no file is
/// configured or the file can't be opened.
fn open_log_file(path: Option<&str>) -> Option<File> {
    let path = path?;
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => Some(file),
        Err(e) => {
            // The subscriber isn't installed yet, so warn via stderr directly.
            eprintln!("doido: could not open log file '{path}': {e}; logging to stdout");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_directives_are_valid() {
        // `try_new` errors on a malformed filter; the defaults must always parse.
        assert!(EnvFilter::try_new(DEFAULT_DIRECTIVES).is_ok());
    }

    #[test]
    fn info_level_matches_default_directives() {
        assert_eq!(directives_for_level("info"), DEFAULT_DIRECTIVES);
    }

    #[test]
    fn level_is_prepended_to_noise_directives() {
        let directives = directives_for_level("debug");
        assert!(directives.starts_with("debug,"));
        assert!(directives.ends_with(NOISE_DIRECTIVES));
        // Whatever the level, the result must remain a valid filter.
        assert!(EnvFilter::try_new(&directives).is_ok());
    }

    #[test]
    fn config_defaults_to_info_and_sql_on() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.sql);
        assert!(config.file.is_none());
        assert_eq!(config.directives(), DEFAULT_DIRECTIVES);
    }

    #[test]
    fn explicit_directives_override_level() {
        let config = LoggerConfig {
            level: "info".to_string(),
            directives: Some("warn,my_app=debug".to_string()),
            ..LoggerConfig::default()
        };
        assert_eq!(config.directives(), "warn,my_app=debug");
    }

    #[test]
    fn default_format_is_compact() {
        assert_eq!(LoggerConfig::default().format, LogFormat::Compact);
    }

    #[test]
    fn json_response_format_isolates_response_target() {
        let config = LoggerConfig {
            format: LogFormat::JsonResponse,
            ..LoggerConfig::default()
        };
        let directives = config.directives();
        assert_eq!(directives, format!("off,{RESPONSE_TARGET}=info"));
        // The isolating filter must still be a valid `EnvFilter`.
        assert!(EnvFilter::try_new(&directives).is_ok());
    }

    #[test]
    fn explicit_directives_win_over_json_response_default() {
        let config = LoggerConfig {
            format: LogFormat::JsonResponse,
            directives: Some("info".to_string()),
            ..LoggerConfig::default()
        };
        assert_eq!(config.directives(), "info");
    }

    #[test]
    fn format_deserializes_from_snake_case() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            format: LogFormat,
        }
        let parsed: Wrapper = serde_norway::from_str("format: json_response\n").unwrap();
        assert_eq!(parsed.format, LogFormat::JsonResponse);
        let parsed: Wrapper = serde_norway::from_str("format: verbose\n").unwrap();
        assert_eq!(parsed.format, LogFormat::Verbose);
    }
}
