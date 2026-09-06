//! Unified runtime localization for backend and view strings.
//!
//! Framework crates register embedded locale files at boot; apps load all
//! `config/locales/*.yml` files (including `{scope}.{locale}.yml`). Process
//! default locale comes from the `DOIDO_LOCALE` environment variable.

mod yaml;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{OnceLock, RwLock};

use crate::Result;

/// Default/fallback locale.
pub const DEFAULT_LOCALE: &str = "en";

/// Environment variable that sets the process-default locale.
pub const LOCALE_ENV_VAR: &str = "DOIDO_LOCALE";

static CATALOG: OnceLock<RwLock<Catalog>> = OnceLock::new();
static TEST_GUARD: OnceLock<std::sync::Mutex<()>> = OnceLock::new();

#[derive(Debug, Default)]
struct Catalog {
    translations: HashMap<String, BTreeMap<String, String>>,
}

fn catalog() -> &'static RwLock<Catalog> {
    CATALOG.get_or_init(|| RwLock::new(Catalog::default()))
}

/// Normalizes assorted locale spellings to catalog keys (`en`, `pt`, `pt_BR`).
/// `pt` → `pt`; `pt-BR`, `pt_br` → `pt_BR`; `en` → `en`; anything else → `en`.
#[must_use]
pub fn normalize_locale(raw: &str) -> &'static str {
    let lower = raw.trim().to_ascii_lowercase().replace('-', "_");
    match lower.as_str() {
        "pt_br" => "pt_BR",
        "pt" => "pt",
        "en" => "en",
        _ => "en",
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

/// Parses a locale filename into optional scope and normalized locale.
///
/// Examples: `en.yml` → `(None, en)`, `auth.en.yml` → `(Some("auth"), en)`,
/// `models.users.pt_BR.yml` → `(Some("models.users"), pt_BR)`.
#[must_use]
pub fn parse_locale_filename(name: &str) -> Option<(Option<String>, &'static str)> {
    let stem = name
        .strip_suffix(".yml")
        .or_else(|| name.strip_suffix(".yaml"))?;

    let parts: Vec<&str> = stem.split('.').collect();
    match parts.len() {
        0 => None,
        1 => Some((None, normalize_locale(parts[0]))),
        n => {
            let locale = normalize_locale(parts[n - 1]);
            let scope = parts[..n - 1].join(".");
            Some((Some(scope), locale))
        }
    }
}

/// Merges YAML locale content into the global catalog.
pub fn register_yaml(locale: &str, yaml: &str, scope: Option<&str>) -> Result<()> {
    let locale = normalize_locale(locale);
    let entries = yaml::parse_yaml(yaml, scope)?;
    merge_entries(locale, entries)
}

/// Registers a single translation key in the global catalog.
pub fn register_entry(locale: &str, key: &str, value: &str) -> Result<()> {
    let mut entries = BTreeMap::new();
    entries.insert(key.to_string(), value.to_string());
    merge_entries(normalize_locale(locale), entries)
}

fn merge_entries(locale: &str, entries: BTreeMap<String, String>) -> Result<()> {
    let mut catalog = catalog()
        .write()
        .map_err(|_| crate::anyhow::anyhow!("i18n catalog lock poisoned"))?;
    let bucket = catalog.translations.entry(locale.to_string()).or_default();
    bucket.extend(entries);
    Ok(())
}

/// Loads every `*.yml` / `*.yaml` file in `path` into the global catalog.
pub fn load_locale_dir(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some((scope, locale)) = parse_locale_filename(&file_name) else {
            continue;
        };
        let content = std::fs::read_to_string(entry.path())?;
        register_yaml(locale, &content, scope.as_deref())?;
    }

    Ok(())
}

/// Loads app locale files and sets the process locale from [`LOCALE_ENV_VAR`].
pub fn init(locales_dir: &Path) -> Result<()> {
    load_locale_dir(locales_dir)?;
    Ok(())
}

/// Installs the process-default locale from [`locale_from_env`] (or `en`) and
/// loads `config/locales/` when present.
pub fn init_from_env() {
    let _ = init(Path::new("config/locales"));
}

/// Translates a message key into the given locale, falling back to [`DEFAULT_LOCALE`].
#[must_use]
pub fn translate(key: &str, locale: &str) -> String {
    let locale = normalize_locale(locale);
    lookup(key, locale)
        .or_else(|| lookup(key, DEFAULT_LOCALE))
        .unwrap_or_else(|| format!("translation missing: {key}"))
}

/// Convenience: translate a key using an optional preferred locale.
#[must_use]
pub fn translate_for(key: &str, preferred: Option<&str>) -> String {
    translate(key, resolve_locale(preferred))
}

/// Translate a key and interpolate `%{name}` placeholders from `vars`.
#[must_use]
pub fn t_with(key: &str, vars: &[(&str, &str)], preferred: Option<&str>) -> String {
    let mut out = translate_for(key, preferred);
    for (name, value) in vars {
        out = out.replace(&format!("%{{{name}}}"), value);
    }
    out
}

fn lookup(key: &str, locale: &str) -> Option<String> {
    catalog()
        .read()
        .ok()?
        .translations
        .get(locale)?
        .get(key)
        .cloned()
}

/// Serializes tests that mutate the global catalog.
#[doc(hidden)]
pub fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_GUARD
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Clears the global catalog. Intended for unit tests.
#[doc(hidden)]
pub fn reset_for_test() {
    if let Ok(mut catalog) = catalog().write() {
        catalog.translations.clear();
    }
}
