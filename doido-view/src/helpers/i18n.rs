//! Translation helper (Rails `t` / i18n).
//!
//! A catalog maps dotted keys to strings with `%{var}` placeholders. `t` looks a
//! key up (returning a `translation missing` marker when absent); `t_with`
//! interpolates variables.

use std::collections::BTreeMap;

/// A translation catalog for one locale.
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

    /// Register a translation for `key`.
    pub fn add(&mut self, key: &str, value: &str) -> &mut Self {
        self.translations.insert(key.to_string(), value.to_string());
        self
    }

    /// Look up `key`, or a `translation missing: <key>` marker if absent.
    pub fn t(&self, key: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| format!("translation missing: {key}"))
    }

    /// Look up `key` and interpolate `%{name}` placeholders from `vars`.
    pub fn t_with(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let mut out = self.t(key);
        for (name, value) in vars {
            out = out.replace(&format!("%{{{name}}}"), value);
        }
        out
    }
}
