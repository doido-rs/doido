pub use sea_orm;
pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, Set,
};

pub mod config;
pub mod create;
pub mod environment;
pub mod migration;
pub mod pool;
pub mod testing;

pub use config::{Config, DatabaseConfig, YamlConfig};
pub use create::create_database;
pub use environment::Environment;

// Rails-style migration helpers: create_table, alter_table, Column::add, …
pub use migration::{
    alter_table, create_table, drop_table, rename_table, AlterTableBuilder, Column, ForeignKey,
    Index, TableBuilder,
};
