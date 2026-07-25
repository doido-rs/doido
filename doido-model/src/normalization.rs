//! Attribute normalization (Rails `normalizes :email, with: -> { ... }`).
//!
//! Build a [`Normalizer`] by chaining steps, then `apply` it to a value —
//! typically inside a `before_save`/`before_validation` callback so the stored
//! value is always canonical.

type Step = Box<dyn Fn(String) -> String + Send + Sync>;

/// An ordered pipeline of string transformations.
#[derive(Default)]
pub struct Normalizer {
    steps: Vec<Step>,
}

impl Normalizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Trim leading and trailing whitespace.
    pub fn strip(mut self) -> Self {
        self.steps.push(Box::new(|s| s.trim().to_string()));
        self
    }

    /// Lowercase the value.
    pub fn downcase(mut self) -> Self {
        self.steps.push(Box::new(|s| s.to_lowercase()));
        self
    }

    /// Uppercase the value.
    pub fn upcase(mut self) -> Self {
        self.steps.push(Box::new(|s| s.to_uppercase()));
        self
    }

    /// Collapse runs of whitespace to a single space and trim (Rails `squish`).
    pub fn squish(mut self) -> Self {
        self.steps.push(Box::new(|s| {
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        }));
        self
    }

    /// Add a custom transformation step.
    pub fn custom(mut self, f: impl Fn(String) -> String + Send + Sync + 'static) -> Self {
        self.steps.push(Box::new(f));
        self
    }

    /// Run every step in order over `input`.
    pub fn apply(&self, input: &str) -> String {
        self.steps
            .iter()
            .fold(input.to_string(), |acc, step| step(acc))
    }
}
