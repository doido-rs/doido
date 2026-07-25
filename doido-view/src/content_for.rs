//! Named content blocks (Rails `content_for(:name) { ... }` + `yield :name`).
//!
//! A view captures HTML under a key with [`ContentFor::set`]; the layout reads it
//! back with [`ContentFor::get`]. Repeated `set`s for a key append, matching
//! Rails' accumulating `content_for`.

use std::collections::BTreeMap;

/// A store of named content blocks for one render.
#[derive(Debug, Default, Clone)]
pub struct ContentFor {
    blocks: BTreeMap<String, String>,
}

impl ContentFor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append `html` to the block under `key`.
    pub fn set(&mut self, key: &str, html: &str) {
        self.blocks
            .entry(key.to_string())
            .or_default()
            .push_str(html);
    }

    /// The captured content for `key` (empty string if none — like `yield`).
    pub fn get(&self, key: &str) -> &str {
        self.blocks.get(key).map(String::as_str).unwrap_or("")
    }

    /// Whether any content was captured for `key` (Rails `content_for?`).
    pub fn has(&self, key: &str) -> bool {
        self.blocks.contains_key(key)
    }
}
