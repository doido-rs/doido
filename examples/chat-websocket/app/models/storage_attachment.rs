//! Model extensions for `storage_attachment` — safe to edit; never overwritten by generators.
//!
//! The SeaORM entity definition lives in `_entities/storage_attachments.rs` and is
//! regenerated on every `doido db migrate`.
#![allow(dead_code, unused_imports)]

pub use super::_entities::storage_attachments::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}
