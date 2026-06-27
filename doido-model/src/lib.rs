pub use sea_orm;
pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, Set,
};

pub mod migration;
pub mod pool;
pub mod testing;

// Rails-style migration objects: Table::create, Column::add, ForeignKey::add, …
pub use migration::{Column, ForeignKey, Index, Table, TableBuilder};
