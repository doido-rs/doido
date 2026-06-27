//! Rails-style schema migration helpers.
//!
//! Every operation is a free function taking the migration's
//! [`SchemaManager`](sea_orm_migration::SchemaManager) as its first argument —
//! table helpers ([`create_table`], [`drop_table`], [`rename_table`],
//! [`alter_table`]), column helpers ([`add_column`], [`remove_column`],
//! [`rename_column`]), index helpers ([`add_index`], [`remove_index`]) and
//! foreign-key helpers ([`add_foreign_key`], [`remove_foreign_key`]). They are
//! meant to be called straight from a `MigrationTrait::up`/`down`:
//!
//! ```no_run
//! # use sea_orm_migration::{SchemaManager, DbErr};
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
mod table;

pub use column::{add_column, remove_column, rename_column};
pub use foreign_key::{add_foreign_key, remove_foreign_key};
pub use index::{add_index, remove_index};
pub use table::{
    alter_table, create_table, drop_table, rename_table, AlterTableBuilder, TableBuilder,
};
