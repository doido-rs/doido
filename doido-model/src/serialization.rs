//! Model serialization helpers: `as_json` with `only`/`except` filtering (Rails
//! `as_json(only:/except:)`) and serialized columns (store a struct as JSON text).

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// Serialize a value to a JSON [`Value`] (null on failure).
pub fn as_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Serialize keeping only the listed top-level keys (Rails `as_json(only:)`).
pub fn as_json_only<T: Serialize>(value: &T, keys: &[&str]) -> Value {
    filter_object(as_json(value), |k| keys.contains(&k))
}

/// Serialize dropping the listed top-level keys (Rails `as_json(except:)`).
pub fn as_json_except<T: Serialize>(value: &T, keys: &[&str]) -> Value {
    filter_object(as_json(value), |k| !keys.contains(&k))
}

fn filter_object(value: Value, keep: impl Fn(&str) -> bool) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.into_iter().filter(|(k, _)| keep(k)).collect()),
        other => other,
    }
}

/// Serialize a value to a JSON string for storage in a text/JSON column.
pub fn serialize_column<T: Serialize>(value: &T) -> doido_core::Result<String> {
    serde_json::to_string(value)
        .map_err(|e| doido_core::anyhow::anyhow!("serialize_column failed: {e}"))
}

/// Deserialize a value previously stored by [`serialize_column`].
pub fn deserialize_column<T: DeserializeOwned>(raw: &str) -> doido_core::Result<T> {
    serde_json::from_str(raw)
        .map_err(|e| doido_core::anyhow::anyhow!("deserialize_column failed: {e}"))
}
