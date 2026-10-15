//! Rails-style schema migration helpers.
//!
//! Every operation is a free function taking the migration's
//! [`SchemaManager`](crate::sea_orm_migration::SchemaManager) as its first argument —
//! table helpers ([`create_table`], [`drop_table`], [`rename_table`],
//! [`alter_table`]), column helpers ([`add_column`], [`remove_column`],
//! [`rename_column`]), index helpers ([`add_index`], [`remove_index`]) and
//! foreign-key helpers ([`add_foreign_key`], [`remove_foreign_key`]). Pass
//! [`CmdOpts`] to the `*_with_opts` variants for explicit `if_not_exists` /
//! `if_exists` behavior; the short helpers default to [`CmdOpts::idempotent`].
//! They are
//! meant to be called straight from a `MigrationTrait::up`/`down`:
//!
//! ```no_run
//! # use doido_model::sea_orm_migration::{SchemaManager, DbErr};
//! # async fn demo(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
//! use doido_model::migration::{create_table, drop_table, alter_table, add_index, add_foreign_key};
//!
//! create_table(manager, "users", |t| {
//!     t.string("email").not_null().unique_key();
//!     t.string("name");
//!     t.timestamps();
//! })
//! .await?;
//!
//! alter_table(manager, "users", |t| {
//!     t.add_column("age", |c| {
//!         c.integer();
//!     });
//!     t.rename_column("name", "full_name");
//! })
//! .await?;
//!
//! add_index(manager, "users", &["email"]).await?;
//! add_foreign_key(manager, "posts", "user_id", "users", "id").await?;
//!
//! drop_table(manager, "users").await?;
//! # Ok(())
//! # }
//! ```

mod column;
mod foreign_key;
mod index;
mod opts;
mod table;

pub use column::{
    add_column, add_column_with_opts, remove_column, remove_column_with_opts, rename_column,
    rename_column_with_opts,
};
pub use foreign_key::{
    add_foreign_key, add_foreign_key_with_opts, remove_foreign_key, remove_foreign_key_with_opts,
};
pub use index::{add_index, add_index_with_opts, remove_index, remove_index_with_opts};
pub use opts::CmdOpts;
pub use table::{
    alter_table, alter_table_with_opts, create_table, create_table_with_opts, drop_table,
    drop_table_with_opts, rename_table, rename_table_with_opts, AlterTableBuilder, TableBuilder,
};
