//! Localization helpers for backend-originated messages.
//!
//! The backend uses `rust-i18n` (initialized in `lib.rs` via `i18n!`). Messages
//! are stored as stable keys (e.g. `auth.invalid_credentials`) and translated
//! here using the best-available locale. Supported locales mirror the
//! frontend/landing_page: `en` (fallback) and `pt_BR`.
//!
//! Process default locale comes from the `DOIDO_LOCALE` environment variable.

use rust_i18n::t;

/// Default/fallback locale.
pub const DEFAULT_LOCALE: &str = "en";

/// Environment variable that sets the process-default locale.
pub const LOCALE_ENV_VAR: &str = "DOIDO_LOCALE";

/// Normalizes assorted locale spellings to the catalog keys (`en`, `pt_BR`).
/// Accepts e.g. `pt`, `pt-BR`, `pt_br`, `PT_BR` → `pt_BR`; anything else → `en`.
#[must_use]
pub fn normalize_locale(raw: &str) -> &'static str {
    let lower = raw.trim().to_ascii_lowercase().replace('-', "_");
    if lower == "pt_br" || lower == "pt" {
        "pt_BR"
    } else {
        "en"
    }
}

/// Reads the process locale from [`LOCALE_ENV_VAR`], if set and non-empty.
#[must_use]
pub fn locale_from_env() -> Option<&'static str> {
    std::env::var(LOCALE_ENV_VAR)
        .ok()
        .filter(|raw| !raw.trim().is_empty())
        .map(|raw| normalize_locale(&raw))
}

/// Picks the best locale from an optional caller-supplied value, then
/// [`locale_from_env`], falling back to [`DEFAULT_LOCALE`].
#[must_use]
pub fn resolve_locale(preferred: Option<&str>) -> &'static str {
    if let Some(raw) = preferred {
        return normalize_locale(raw);
    }
    locale_from_env().unwrap_or(DEFAULT_LOCALE)
}

/// Translates a message key into the given locale.
#[must_use]
pub fn translate(key: &str, locale: &str) -> String {
    t!(key, locale = locale).to_string()
}

/// Convenience: translate a key using an optional preferred locale.
#[must_use]
pub fn translate_for(key: &str, preferred: Option<&str>) -> String {
    translate(key, resolve_locale(preferred))
}

/// Installs the process-default locale from [`locale_from_env`] (or `en`).
pub fn init_from_env() {
    rust_i18n::set_locale(resolve_locale(None));
}
