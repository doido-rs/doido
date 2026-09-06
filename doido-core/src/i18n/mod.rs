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

/// Normalizes assorted locale spellings to a canonical catalog key.
///
/// Language tags are lowercased; region subtags are uppercased and joined with
/// an underscore (`pt-BR` → `pt_BR`, `fr-FR` → `fr_FR`). Does not remap unknown
/// locales to [`DEFAULT_LOCALE`].
#[must_use]
pub fn normalize_locale(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_LOCALE.to_string();
    }

    let parts: Vec<&str> = trimmed
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .collect();

    match parts.as_slice() {
        [] => DEFAULT_LOCALE.to_string(),
        [lang] => lang.to_ascii_lowercase(),
        [lang, region] => format!(
            "{}_{}",
            lang.to_ascii_lowercase(),
            region.to_ascii_uppercase()
        ),
        [lang, region, ..] => format!(
            "{}_{}",
            lang.to_ascii_lowercase(),
            region.to_ascii_uppercase()
        ),
    }
}

/// Locales currently loaded in the global catalog.
#[must_use]
pub fn available_locales() -> Vec<String> {
    catalog()
        .read()
        .ok()
        .map(|catalog| {
            let mut locales: Vec<_> = catalog.translations.keys().cloned().collect();
            locales.sort();
            locales
        })
        .unwrap_or_default()
}

/// Whether `locale` (after normalization) exists in the global catalog.
#[must_use]
pub fn locale_available(locale: &str) -> bool {
    let locale = normalize_locale(locale);
    catalog()
        .read()
        .ok()
        .is_some_and(|catalog| catalog.translations.contains_key(&locale))
}

/// Reads the process locale from [`LOCALE_ENV_VAR`], if set and non-empty.
#[must_use]
pub fn locale_from_env() -> Option<String> {
    std::env::var(LOCALE_ENV_VAR)
        .ok()
        .filter(|raw| !raw.trim().is_empty())
        .map(|raw| normalize_locale(&raw))
}

/// Picks the best locale from an optional caller-supplied value, then
/// [`locale_from_env`], falling back to [`DEFAULT_LOCALE`].
///
/// Fails when the resolved locale is not present in the loaded catalog.
pub fn resolve_locale(preferred: Option<&str>) -> Result<String> {
    let raw = preferred
        .map(str::to_string)
        .or_else(locale_from_env)
        .unwrap_or_else(|| DEFAULT_LOCALE.to_string());
    let locale = normalize_locale(&raw);
    if locale_available(&locale) {
        Ok(locale)
    } else {
        Err(crate::anyhow::anyhow!(locale_not_available_message(
            &raw, &locale
        )))
    }
}

/// Parses a locale filename into optional scope and normalized locale.
///
/// Examples: `en.yml` → `(None, en)`, `auth.en.yml` → `(Some("auth"), en)`,
/// `models.users.pt_BR.yml` → `(Some("models.users"), pt_BR)`.
#[must_use]
pub fn parse_locale_filename(name: &str) -> Option<(Option<String>, String)> {
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
    merge_entries(&locale, entries)
}

/// Registers a single translation key in the global catalog.
pub fn register_entry(locale: &str, key: &str, value: &str) -> Result<()> {
    let mut entries = BTreeMap::new();
    entries.insert(key.to_string(), value.to_string());
    merge_entries(&normalize_locale(locale), entries)
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
        register_yaml(&locale, &content, scope.as_deref())?;
    }

    Ok(())
}

/// Loads app locale files and validates the active process locale.
pub fn init(locales_dir: &Path) -> Result<()> {
    load_locale_dir(locales_dir)?;
    validate_active_locale()
}

/// Loads `config/locales/` when present and validates the active locale.
pub fn init_from_env() {
    let _ = init(Path::new("config/locales"));
}

fn validate_active_locale() -> Result<()> {
    resolve_locale(None).map(|_| ())
}

fn locale_not_available_message(raw: &str, locale: &str) -> String {
    let available = available_locales();
    if available.is_empty() {
        format!("locale not available: {raw} (normalized: {locale}; no locales loaded)")
    } else {
        format!(
            "locale not available: {raw} (normalized: {locale}; available: {})",
            available.join(", ")
        )
    }
}

/// Translates a message key into the given locale.
///
/// Returns an error message when the locale is not loaded. Missing keys fall
/// back to [`DEFAULT_LOCALE`] when that locale is available.
#[must_use]
pub fn translate(key: &str, locale: &str) -> String {
    let locale = normalize_locale(locale);
    if !locale_available(&locale) {
        return locale_not_available_message(&locale, &locale);
    }

    lookup(key, &locale)
        .or_else(|| {
            if locale != DEFAULT_LOCALE && locale_available(DEFAULT_LOCALE) {
                lookup(key, DEFAULT_LOCALE)
            } else {
                None
            }
        })
        .unwrap_or_else(|| format!("translation missing: {key}"))
}

/// Convenience: translate a key using an optional preferred locale.
#[must_use]
pub fn translate_for(key: &str, preferred: Option<&str>) -> String {
    match resolve_locale(preferred) {
        Ok(locale) => translate(key, &locale),
        Err(error) => error.to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_locale_canonicalizes_language_tags() {
        assert_eq!(normalize_locale("pt_BR"), "pt_BR");
        assert_eq!(normalize_locale("pt-BR"), "pt_BR");
        assert_eq!(normalize_locale("PT_br"), "pt_BR");
        assert_eq!(normalize_locale("pt"), "pt");
        assert_eq!(normalize_locale("pt-PT"), "pt_PT");
        assert_eq!(normalize_locale("pt_pt"), "pt_PT");
        assert_eq!(normalize_locale("en"), "en");
        assert_eq!(normalize_locale("fr"), "fr");
        assert_eq!(normalize_locale("fr-FR"), "fr_FR");
    }
}
