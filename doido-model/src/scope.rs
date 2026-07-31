//! Named, chainable query scopes (Rails `scope :published, -> { where(...) }`).
//!
//! A scope is any query transformation `FnOnce(Select<E>) -> Select<E>` — a free
//! function or a closure. [`Scoped`] lets them chain fluently off a sea-orm
//! `Select`, e.g. `Entity::find().scope(published).scope(recent)`.

use crate::sea_orm::{EntityTrait, Select};

/// Fluent application of named scopes to a query.
pub trait Scoped: Sized {
    /// Apply a scope (a query transformation).
    fn scope(self, transform: impl FnOnce(Self) -> Self) -> Self {
        transform(self)
    }

    /// Apply a scope only when `condition` holds (Rails conditional scope).
    fn scope_if(self, condition: bool, transform: impl FnOnce(Self) -> Self) -> Self {
        if condition {
            transform(self)
        } else {
            self
        }
    }
}

impl<E: EntityTrait> Scoped for Select<E> {}
