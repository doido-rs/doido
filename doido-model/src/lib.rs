pub use sea_orm;
pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, Set,
};
#[cfg(feature = "cli")]
pub use sea_orm_cli;
pub use sea_orm_migration;
pub use sea_orm_migration::SchemaManager;

pub mod association;
pub mod callbacks;
#[cfg(feature = "cli")]
pub mod commands;
pub mod config;
pub mod create;
pub mod databases;
pub mod entities;
pub mod enums;
pub mod factory;
pub mod migrate;
pub mod migration;
pub mod normalization;
pub mod password;
pub mod pool;
pub mod schema;
#[cfg(feature = "cli")]
pub mod schema_design;
pub mod scope;
pub mod seeds;
pub mod serialization;
pub mod tasks;
pub mod testing;
pub mod transaction;
pub mod validation;

pub use config::{Config, DatabaseConfig, LoggerConfig, YamlConfig};
pub use create::create_database;
pub use pool::{connect, connect_with_url};

#[cfg(feature = "cli")]
pub use schema_design::{export_html, introspect_from_url, resolve_ignore_tables, SchemaDesign};

// Rails-style migration helpers: create_table, alter_table, add_column, …
pub use migration::{
    add_column, add_foreign_key, add_index, alter_table, create_table, drop_table, remove_column,
    remove_foreign_key, remove_index, rename_column, rename_table, AlterTableBuilder, TableBuilder,
};
