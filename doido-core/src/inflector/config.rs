//! Project-level inflection overrides loaded from `config/inflection.yaml`.
//!
//! The file layers custom rules on top of the default English rules. Every
//! section is optional:
//!
//! ```yaml
//! # config/inflection.yaml
//! irregulars:
//!   - { singular: goose, plural: geese }
//!   - { singular: person, plural: people }
//! uncountables:
//!   - bitcoin
//!   - equipment
//! acronyms:
//!   - API
//!   - HTTP
//! plurals:
//!   - { pattern: "(quiz)$", replacement: "${1}zes" }
//! singulars:
//!   - { pattern: "(quiz)zes$", replacement: "${1}" }
//! ```

use serde::Deserialize;

use super::Inflections;

/// A regex-based plural/singular rule: `pattern` is matched (case-insensitively
/// at use time) and rewritten with `replacement` (supports `${1}` captures).
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub pattern: String,
    pub replacement: String,
}

/// An irregular word pair, e.g. `person` ↔ `people`.
#[derive(Debug, Clone, Deserialize)]
pub struct Irregular {
    pub singular: String,
    pub plural: String,
}

/// Deserialized form of `config/inflection.yaml`. Unknown keys are rejected so
/// typos surface instead of being silently ignored.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InflectionConfig {
    pub plurals: Vec<Rule>,
    pub singulars: Vec<Rule>,
    pub irregulars: Vec<Irregular>,
    pub uncountables: Vec<String>,
    pub acronyms: Vec<String>,
}

impl InflectionConfig {
    /// Parse a YAML document into a config.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_norway::Error> {
        serde_norway::from_str(yaml)
    }

    /// Apply these overrides onto an existing `Inflections` set. Rules added here
    /// take priority over the defaults already present (last-added wins).
    pub fn apply(&self, inflections: &mut Inflections) {
        for rule in &self.plurals {
            inflections.plural(&rule.pattern, &rule.replacement);
        }
        for rule in &self.singulars {
            inflections.singular(&rule.pattern, &rule.replacement);
        }
        for irr in &self.irregulars {
            inflections.irregular(&irr.singular, &irr.plural);
        }
        for word in &self.uncountables {
            inflections.uncountable(word);
        }
        for word in &self.acronyms {
            inflections.acronym(word);
        }
    }
}
