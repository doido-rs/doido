//! Environment-variable config overrides (spec 05 `SECTION__KEY`).
//!
//! An env var named `SECTION__KEY` (double underscore) overrides
//! `config[section][key]`, so `SERVER__PORT=4000` sets `server.port`. Values are
//! coerced to bool/number when they parse, else kept as strings.

use serde_json::Value;

/// Apply `SECTION__KEY=value` overrides (as `(name, value)` pairs) onto `config`.
pub fn apply_env_overrides(config: &mut Value, vars: &[(String, String)]) {
    let Value::Object(root) = config else { return };
    for (name, raw) in vars {
        let Some((section, key)) = name.split_once("__") else {
            continue;
        };
        let section = section.to_lowercase();
        let key = key.to_lowercase();
        let entry = root
            .entry(section)
            .or_insert_with(|| Value::Object(Default::default()));
        if let Value::Object(map) = entry {
            map.insert(key, coerce(raw));
        }
    }
}

/// Read `SECTION__KEY` overrides from the process environment.
pub fn from_process_env(config: &mut Value) {
    let vars: Vec<(String, String)> = std::env::vars().filter(|(k, _)| k.contains("__")).collect();
    apply_env_overrides(config, &vars);
}

fn coerce(raw: &str) -> Value {
    if let Ok(b) = raw.parse::<bool>() {
        return Value::Bool(b);
    }
    if let Ok(i) = raw.parse::<i64>() {
        return Value::from(i);
    }
    if let Ok(f) = raw.parse::<f64>() {
        return Value::from(f);
    }
    Value::String(raw.to_string())
}
