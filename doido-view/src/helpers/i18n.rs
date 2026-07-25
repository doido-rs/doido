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

impl I18n {
    /// Load a YAML locale file (`locale:\n  key: value\n  nested:\n    k: v`)
    /// into the catalog, flattening nested keys with dots (`nested.k`).
    pub fn load_yaml(&mut self, yaml: &str) -> doido_core::Result<&mut Self> {
        let value: serde_json::Value = serde_norway::from_str(yaml)
            .map_err(|e| doido_core::anyhow::anyhow!("locale parse failed: {e}"))?;
        if let Some(root) = value.as_object() {
            for subtree in root.values() {
                flatten("", subtree, &mut self.translations);
            }
        }
        Ok(self)
    }
}

fn flatten(prefix: &str, value: &serde_json::Value, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let full = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(&full, child, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}
