//! Active Record-style validations.
//!
//! Rust models have no shared base class, so a model opts in by implementing
//! [`Validate`] and building an [`Errors`] with the provided validators
//! (`presence`, `length`, …) or ad-hoc [`Errors::add`] calls.

/// A collection of `(field, message)` validation errors (Rails `model.errors`).
#[derive(Debug, Default, Clone)]
pub struct Errors {
    items: Vec<(String, String)>,
}

impl Errors {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a raw error message for `field`.
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.items.push((field.into(), message.into()));
        self
    }

    /// Whether there are no errors (the record is valid).
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The number of errors.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// The messages recorded for a specific field.
    pub fn on(&self, field: &str) -> Vec<&str> {
        self.items
            .iter()
            .filter(|(f, _)| f == field)
            .map(|(_, m)| m.as_str())
            .collect()
    }

    /// Human-readable `"field message"` strings for every error.
    pub fn full_messages(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|(field, message)| format!("{field} {message}"))
            .collect()
    }

    /// Require a non-blank string (Rails `validates :field, presence: true`).
    pub fn presence(&mut self, field: &str, value: &str) -> &mut Self {
        if value.trim().is_empty() {
            self.add(field, "can't be blank");
        }
        self
    }

    /// Constrain a string's character length (Rails `length: { minimum:, maximum: }`).
    pub fn length(
        &mut self,
        field: &str,
        value: &str,
        min: Option<usize>,
        max: Option<usize>,
    ) -> &mut Self {
        let len = value.chars().count();
        if let Some(min) = min {
            if len < min {
                self.add(field, format!("is too short (minimum is {min} characters)"));
            }
        }
        if let Some(max) = max {
            if len > max {
                self.add(field, format!("is too long (maximum is {max} characters)"));
            }
        }
        self
    }
}

/// Implemented by models that can validate themselves.
pub trait Validate {
    /// Collect validation errors for the current state.
    fn validate(&self) -> Errors;

    /// Whether the record currently passes validation.
    fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }
}
