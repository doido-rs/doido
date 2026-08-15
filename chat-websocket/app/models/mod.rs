//! Application models. Each `doido generate model <Name>` registers its module
//! here, just above the marker below.
//!
//! Entity structs live under `_entities/` (regenerated on migrate). Edit the
//! sibling `app/models/<name>.rs` files for validations and custom behaviour.

pub mod _entities;

pub mod user;
// @generated-models
