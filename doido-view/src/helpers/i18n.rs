//! Translation helper (Rails `t` / i18n).
//!
//! Delegates to the unified [`doido_core::i18n`] catalog loaded at boot from
//! framework crates and `config/locales/*.yml`.

use std::collections::BTreeMap;

/// Look up `key` in the global catalog.
#[must_use]
pub fn t(key: &str) -> String {
    doido_core::i18n::translate_for(key, None)
}

/// Look up `key` and interpolate `%{name}` placeholders from `vars`.
#[must_use]
pub fn t_with(key: &str, vars: &[(&str, &str)]) -> String {
    doido_core::i18n::t_with(key, vars, None)
}

/// A translation catalog wrapper for one locale.
///
/// New code should prefer [`t`] and [`t_with`]; this type remains for backwards
/// compatibility and tests.
#[derive(Debug, Default, Clone)]
pub struct I18n {
    locale: String,
    translations: BTreeMap<String, String>,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        Self {
            locale: locale.to_string(),
            translations: BTreeMap::new(),
        }
    }

    /// The catalog's locale (e.g. `"en"`).
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Register a translation for `key` in the global catalog.
    pub fn add(&mut self, key: &str, value: &str) -> &mut Self {
        self.translations.insert(key.to_string(), value.to_string());
        let _ = doido_core::i18n::register_entry(self.locale(), key, value);
        self
    }

    /// Look up `key` in the global catalog.
    pub fn t(&self, key: &str) -> String {
        t(key)
    }

    /// Look up `key` and interpolate `%{name}` placeholders from `vars`.
    pub fn t_with(&self, key: &str, vars: &[(&str, &str)]) -> String {
        t_with(key, vars)
    }
}

impl I18n {
    /// Load a YAML locale file into the global catalog for this instance's locale.
    pub fn load_yaml(&mut self, yaml: &str) -> doido_core::Result<&mut Self> {
        doido_core::i18n::register_yaml(self.locale(), yaml, None)?;
        Ok(self)
    }
}
