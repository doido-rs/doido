//! Rails-style schema migration helpers.
//!
//! Every operation is a free function taking the migration's
//! [`SchemaManager`](crate::sea_orm_migration::SchemaManager) as its first argument.
//! Prefer these builders and SeaQuery over `execute_unprepared` / string SQL in app
//! migrations.
//!
//! Table helpers: [`create_table`], [`drop_table`], [`drop_table_cascade`],
//! [`rename_table`], [`alter_table`], [`create_audit_version_table`].
//!
//! Column helpers: [`add_column`], [`remove_column`], [`rename_column`].
//!
//! Index helpers: [`add_index`], [`remove_index`], [`create_index`], [`drop_index`].
//!
//! Foreign-key helpers: [`add_foreign_key`], [`remove_foreign_key`],
//! [`add_foreign_key_with_opts`], [`remove_foreign_key_by_name`].
//!
//! Pass [`CmdOpts`] to the `*_with_opts` / `*_with_cmd_opts` variants for explicit
//! `if_not_exists` / `if_exists` behavior; the short helpers default to
//! [`CmdOpts::idempotent`].
//!
//! Data helpers ([`data`]): [`update_from`], [`update_where`], [`delete_where`].
//!
//! PostgreSQL: [`enable_extension`], [`enum_type::create_enum_type`], …
//!
//! ```no_run
//! # use doido_model::sea_orm_migration::{SchemaManager, DbErr};
//! # async fn demo(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
//! use doido_model::migration::{
//!     create_table, drop_table, alter_table, add_index, add_foreign_key, enable_extension,
//!     create_index, IndexColumn, IndexOpts,
//! };
//! use doido_model::sea_orm::sea_query::{Expr, ExprTrait};
//!
//! enable_extension(manager, "pgcrypto").await?;
//!
//! create_table(manager, "users", |t| {
//!     t.string("email").not_null().unique_key();
//!     t.timestamps();
//! })
//! .await?;
//!
//! create_index(
//!     manager,
//!     "users",
//!     &[IndexColumn::new("email")],
//!     IndexOpts {
//!         unique: true,
//!         where_expr: Some(Expr::col("email").is_not_null()),
//!         ..Default::default()
//!     },
//! )
//! .await?;
//!
//! alter_table(manager, "users", |t| {
//!     t.set_not_null("email");
//! })
//! .await?;
//!
//! add_foreign_key(manager, "posts", "user_id", "users", "id").await?;
//! drop_table(manager, "users").await?;
//! # Ok(())
//! # }
//! ```

mod audit;
mod column;
mod data;
mod enum_type;
mod extension;
mod foreign_key;
mod index;
mod opts;
mod table;

pub use audit::{create_audit_version_table, drop_audit_version_table};
pub use column::{
    add_column, add_column_with_opts, remove_column, remove_column_with_opts, rename_column,
    rename_column_with_opts,
};
pub use data::{delete_where, update_from, update_where};
pub use enum_type::{add_enum_values, create_enum_type, drop_enum_type};
pub use extension::enable_extension;
pub use foreign_key::{
    add_foreign_key, add_foreign_key_with_cmd_opts, add_foreign_key_with_opts,
    default_foreign_key_name, remove_foreign_key, remove_foreign_key_by_name,
    remove_foreign_key_by_name_with_opts, remove_foreign_key_with_opts, ForeignKeyOpts,
};
pub use index::{
    add_index, add_index_with_opts, create_index, default_index_name, drop_index, remove_index,
    remove_index_with_opts, IndexColumn, IndexOpts, IndexSort,
};
pub use opts::CmdOpts;
pub use table::{
    alter_table, alter_table_with_opts, create_table, create_table_with_opts, drop_table,
    drop_table_cascade, drop_table_with_opts, rename_table, rename_table_with_opts,
    AlterTableBuilder, ReferenceOpts, TableBuilder,
};
