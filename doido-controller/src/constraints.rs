//! Route/param constraints (Rails `constraints: { id: /\d+/ }`).
//!
//! Axum's path matcher has no regex constraints, so constraints are enforced as
//! a check — typically from a `#[before_action]` that 404s (or 400s) when a
//! path param fails its format. [`Constraints`] maps param names to validators.

/// Non-empty and all ASCII digits.
pub fn numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Non-empty and all ASCII letters.
pub fn alpha(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Non-empty and all ASCII letters or digits.
pub fn alphanumeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// The canonical 8-4-4-4-12 hex UUID shape.
pub fn uuid_like(s: &str) -> bool {
    let groups = [8, 4, 4, 4, 12];
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == groups.len()
        && parts
            .iter()
            .zip(groups)
            .all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// A set of `param name → validator` rules.
#[derive(Default)]
pub struct Constraints {
    rules: Vec<(String, fn(&str) -> bool)>,
}

impl Constraints {
    pub fn new() -> Self {
        Self::default()
    }

    /// Require `name` to satisfy `validator`.
    pub fn param(mut self, name: &str, validator: fn(&str) -> bool) -> Self {
        self.rules.push((name.to_string(), validator));
        self
    }

    /// Whether every rule is satisfied by `params` (a rule's param must be
    /// present and valid; unruled params are ignored).
    pub fn matches(&self, params: &[(&str, &str)]) -> bool {
        self.rules.iter().all(|(name, validator)| {
            params
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| validator(v))
                .unwrap_or(false)
        })
    }
}
