rust_i18n::i18n!("locales", fallback = "en");

pub mod commands;
pub mod concerns;
pub mod core_ext;
pub mod crypto;
pub mod environment;
pub mod error;
pub mod i18n;
pub mod inflector;
pub mod logger;
pub mod notifications;
pub mod test_time;
pub mod time_ext;
pub mod trace;

// Convenience re-exports so downstream crates depend only on doido-core.
pub use ::anyhow;
pub use ::async_trait::async_trait;
pub use ::serde;
pub use ::thiserror;
pub use ::tracing;

pub use environment::Environment;
pub use error::{AnyhowContext, Result};
pub use i18n::{
    init_from_env, locale_from_env, normalize_locale, resolve_locale, translate, translate_for,
    DEFAULT_LOCALE, LOCALE_ENV_VAR,
};
pub use inflector::{init_inflections, load_inflections, InflectionConfig, Inflections, Inflector};
pub use logger::init as init_logger;
pub use logger::{LogFormat, LoggerConfig};
