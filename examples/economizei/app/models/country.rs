//! Model extensions for `country` — safe to edit; never overwritten by generators.
#![allow(dead_code, unused_imports)]

pub use super::_entities::countries::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}
