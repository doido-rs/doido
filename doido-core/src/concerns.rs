//! The concern/mixin pattern (Rails `ActiveSupport::Concern`).
//!
//! A concern is a trait that provides shared behavior via default methods; a
//! type "includes" it by implementing the small required surface. [`Sluggable`]
//! is a worked example: give it a source string, get a URL slug for free.

/// A reusable concern: derive a URL slug from a source string.
pub trait Sluggable {
    /// The text a slug is derived from (e.g. a title).
    fn sluggable_source(&self) -> &str;

    /// The slug: lowercased, whitespace collapsed to single dashes.
    fn slug(&self) -> String {
        self.sluggable_source()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-")
    }
}
