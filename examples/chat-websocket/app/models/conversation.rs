//! Model extensions for Conversation — safe to edit; never overwritten by generators.
//!
//! The SeaORM entity definition lives in `_entities/conversations.rs` and is
//! regenerated on every `doido db migrate`.
#![allow(dead_code, unused_imports)]

pub use super::_entities::conversations::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}
