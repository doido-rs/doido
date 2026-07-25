pub use sea_orm;
pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, Set,
};
pub use sea_orm_migration;
pub use sea_orm_migration::SchemaManager;

pub mod association;
pub mod callbacks;
pub mod config;
pub mod create;
pub mod databases;
pub mod enums;
pub mod environment;
pub mod factory;
pub mod migration;
pub mod normalization;
pub mod password;
pub mod pool;
pub mod schema;
pub mod scope;
pub mod seeds;
pub mod serialization;
pub mod testing;
pub mod transaction;
pub mod validation;

pub use config::{Config, DatabaseConfig, LoggerConfig, YamlConfig};
pub use create::create_database;
pub use environment::Environment;
pub use pool::{connect, connect_with_url};

// Rails-style migration helpers: create_table, alter_table, add_column, …
pub use migration::{
    add_column, add_foreign_key, add_index, alter_table, create_table, drop_table, remove_column,
    remove_foreign_key, remove_index, rename_column, rename_table, AlterTableBuilder, TableBuilder,
};
