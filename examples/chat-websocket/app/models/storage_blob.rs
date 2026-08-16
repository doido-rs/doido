//! Model extensions for `storage_blob` — safe to edit; never overwritten by generators.
//!
//! The SeaORM entity definition lives in `_entities/storage_blobs.rs` and is
//! regenerated on every `doido db migrate`.
#![allow(dead_code, unused_imports)]

pub use super::_entities::storage_blobs::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}
