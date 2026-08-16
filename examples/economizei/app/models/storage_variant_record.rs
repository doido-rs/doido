//! Model extensions for `storage_variant_record` — safe to edit; never overwritten by generators.
//!
//! The SeaORM entity definition lives in `_entities/storage_variant_records.rs` and is
//! regenerated on every `doido db migrate`.
#![allow(dead_code, unused_imports)]

pub use super::_entities::storage_variant_records::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}
