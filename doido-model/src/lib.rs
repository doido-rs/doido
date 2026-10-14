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
    add_column, add_column_with_opts, add_foreign_key, add_foreign_key_with_opts, add_index,
    add_index_with_opts, alter_table, alter_table_with_opts, create_table, create_table_with_opts,
    drop_table, drop_table_with_opts, remove_column, remove_column_with_opts, remove_foreign_key,
    remove_foreign_key_with_opts, remove_index, remove_index_with_opts, rename_column,
    rename_column_with_opts, rename_table, rename_table_with_opts, AlterTableBuilder, CmdOpts,
    TableBuilder,
};
