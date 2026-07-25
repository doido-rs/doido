//! Attribute enums with helpers (Rails `enum status: { ... }`).
//!
//! A real Rust enum already gives you variant comparison (`status ==
//! Status::Active` is Rails' `status.active?`). [`AttributeEnum`] adds the rest:
//! a stable string `label` per variant, enumeration of `all` variants, and
//! `from_label`/`index`/`labels` for storage and UI.

/// Implemented by enum attributes to gain label mapping and enumeration.
pub trait AttributeEnum: Sized + Copy + PartialEq + 'static {
    /// All variants, in declaration order.
    fn all() -> &'static [Self];

    /// The stable string label for this variant (e.g. `"active"`).
    fn label(&self) -> &'static str;

    /// Parse a variant from its label.
    fn from_label(label: &str) -> Option<Self> {
        Self::all().iter().copied().find(|v| v.label() == label)
    }

    /// The variant's position in [`all`](Self::all) (its integer mapping).
    fn index(&self) -> usize {
        Self::all().iter().position(|v| v == self).unwrap_or(0)
    }

    /// Every variant's label, in order.
    fn labels() -> Vec<&'static str> {
        Self::all().iter().map(|v| v.label()).collect()
    }
}
