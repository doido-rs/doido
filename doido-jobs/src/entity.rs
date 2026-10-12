//! Internal SeaORM entity for the `doido_jobs` queue table.

use doido_model::sea_orm as sea_orm;
use doido_model::sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "doido_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub queue: String,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    pub priority: i64,
    pub run_at: i64,
    pub locked_at: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub data: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
