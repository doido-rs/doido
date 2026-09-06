use std::collections::BTreeMap;

use crate::Result;

/// Parses YAML locale content into flat dotted keys for one locale.
pub fn parse_yaml(yaml: &str, scope: Option<&str>) -> Result<BTreeMap<String, String>> {
    let value: serde_json::Value = serde_norway::from_str(yaml)
        .map_err(|e| crate::anyhow::anyhow!("locale parse failed: {e}"))?;

    let Some(mut root) = value.as_object().cloned() else {
        return Ok(BTreeMap::new());
    };

    root.remove("_version");

    let content = unwrap_locale_root(&root);
    let mut flat = BTreeMap::new();
    flatten("", &content, &mut flat);
    apply_scope(scope, &mut flat);
    Ok(flat)
}

/// If the YAML root is a single locale wrapper (`en:`, `pt-BR:`), return its inner object.
fn unwrap_locale_root(root: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    if root.len() == 1 {
        if let Some((key, value)) = root.iter().next() {
            if looks_like_locale_key(key) && value.is_object() {
                return value.clone();
            }
        }
    }

    serde_json::Value::Object(root.clone())
}

/// Heuristic: locale segment is short and alphabetic (with optional region).
fn looks_like_locale_key(key: &str) -> bool {
    let trimmed = key.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 10
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '_')
}

fn apply_scope(scope: Option<&str>, flat: &mut BTreeMap<String, String>) {
    let Some(scope) = scope.filter(|s| !s.is_empty()) else {
        return;
    };

    let prefix = format!("{scope}.");
    if flat
        .keys()
        .any(|key| key.starts_with(&prefix) || key == scope)
    {
        return;
    }

    let prefixed = std::mem::take(flat)
        .into_iter()
        .map(|(key, value)| (format!("{prefix}{key}"), value))
        .collect();
    *flat = prefixed;
}

fn flatten(prefix: &str, value: &serde_json::Value, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key == "_version" {
                    continue;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_rails_locale_root() {
        let yaml = "en:\n  hello: Hi\n  user:\n    greeting: Welcome\n";
        let flat = parse_yaml(yaml, None).unwrap();
        assert_eq!(flat.get("hello"), Some(&"Hi".to_string()));
        assert_eq!(flat.get("user.greeting"), Some(&"Welcome".to_string()));
    }

    #[test]
    fn applies_scope_prefix_when_missing() {
        let yaml = "_version: 1\ninvalid_credentials: Nope\n";
        let flat = parse_yaml(yaml, Some("auth")).unwrap();
        assert_eq!(
            flat.get("auth.invalid_credentials"),
            Some(&"Nope".to_string())
        );
    }

    #[test]
    fn skips_scope_prefix_when_already_present() {
        let yaml = "auth:\n  invalid_credentials: Nope\n";
        let flat = parse_yaml(yaml, Some("auth")).unwrap();
        assert_eq!(
            flat.get("auth.invalid_credentials"),
            Some(&"Nope".to_string())
        );
    }
}
