//! Strong parameters: a Rails-style allowlist over untyped request params.
//!
//! Deserializing straight into a typed struct is the common path in doido, but
//! when you hold an untyped params object (e.g. a nested form) [`Params`] lets
//! you `require` a key and `permit` an explicit set of fields before use, so
//! unexpected keys can never be mass-assigned.

use doido_core::Result;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// A wrapper around a JSON params object supporting `require`/`permit`.
#[derive(Debug, Clone)]
pub struct Params {
    value: Value,
}

impl Params {
    /// Wrap a params value (typically a JSON object).
    pub fn new(value: Value) -> Self {
        Self { value }
    }

    /// Return the nested params under `key`, erroring if it is absent (Rails
    /// `params.require(:key)`).
    pub fn require(&self, key: &str) -> Result<Params> {
        match self.value.get(key) {
            Some(v) => Ok(Params::new(v.clone())),
            None => Err(doido_core::anyhow::anyhow!(
                "param `{key}` is required but missing"
            )),
        }
    }

    /// Keep only the listed top-level keys, dropping everything else (Rails
    /// `params.permit(:a, :b)`). Non-object params become an empty object.
    pub fn permit(&self, allowed: &[&str]) -> Params {
        let mut out = Map::new();
        if let Value::Object(map) = &self.value {
            for &key in allowed {
                if let Some(v) = map.get(key) {
                    out.insert(key.to_string(), v.clone());
                }
            }
        }
        Params::new(Value::Object(out))
    }

    /// Borrow a raw value by key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.value.get(key)
    }

    /// Deserialize the (typically permitted) params into a typed value.
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_value(self.value.clone())
            .map_err(|e| doido_core::anyhow::anyhow!("params deserialization failed: {e}"))
    }

    /// Consume into the underlying JSON value.
    pub fn into_value(self) -> Value {
        self.value
    }
}
