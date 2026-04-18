pub fn apply_env_overrides(value: &mut toml::Value) {
    apply_overrides_from(value, std::env::vars());
}

pub fn apply_overrides_from(
    value: &mut toml::Value,
    vars: impl Iterator<Item = (String, String)>,
) {
    for (key, val_str) in vars {
        if let Some(path) = parse_env_key(&key) {
            set_nested(value, &path, coerce_value(val_str));
        }
    }
}

/// Converts `SECTION__KEY` or `A__B__C` into `["section", "key"]` / `["a", "b", "c"]`.
/// Returns `None` if the key has no `__` or has empty segments.
fn parse_env_key(key: &str) -> Option<Vec<String>> {
    if !key.contains("__") {
        return None;
    }
    let parts: Vec<String> = key.split("__").map(|s| s.to_lowercase()).collect();
    if parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    Some(parts)
}

/// Tries to parse the string as i64, then f64, then bool; falls back to String.
fn coerce_value(s: String) -> toml::Value {
    if let Ok(n) = s.parse::<i64>() {
        return toml::Value::Integer(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return toml::Value::Float(f);
    }
    match s.to_lowercase().as_str() {
        "true" => return toml::Value::Boolean(true),
        "false" => return toml::Value::Boolean(false),
        _ => {}
    }
    toml::Value::String(s)
}

fn set_nested(value: &mut toml::Value, path: &[String], val: toml::Value) {
    if let toml::Value::Table(map) = value {
        if path.len() == 1 {
            map.insert(path[0].clone(), val);
        } else {
            let child = map
                .entry(path[0].clone())
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
            set_nested(child, &path[1..], val);
        }
    }
}
