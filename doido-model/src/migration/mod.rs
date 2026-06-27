//! Rails-style schema migration helpers.
//!
//! Table operations are free functions — [`create_table`], [`drop_table`],
//! [`rename_table`], [`alter_table`]. Column, index and foreign-key helpers are
//! grouped under [`Column`], [`Index`] and [`ForeignKey`]. All run against any
//! sea-orm [`ConnectionTrait`]:
//!
//! ```no_run
//! # async fn demo(db: &impl sea_orm::ConnectionTrait) -> Result<(), sea_orm::DbErr> {
//! use doido_model::migration::{create_table, drop_table, alter_table, Index, ForeignKey};
//!
//! create_table(db, "users", |t| {
//!     t.string("email").not_null().unique_key();
//!     t.string("name");
//!     t.timestamps();
//! })
//! .await?;
//!
//! alter_table(db, "users", |t| {
//!     t.add_column("age", |c| {
//!         c.integer();
//!     });
//!     t.rename_column("name", "full_name");
//! })
//! .await?;
//!
//! Index::add(db, "users", &["email"]).await?;
//! ForeignKey::add(db, "posts", "user_id", "users", "id").await?;
//!
//! drop_table(db, "users").await?;
//! # Ok(())
//! # }
//! ```

mod column;
mod foreign_key;
mod index;
mod table;

pub use column::Column;
pub use foreign_key::ForeignKey;
pub use index::Index;
pub use table::{
    alter_table, create_table, drop_table, rename_table, AlterTableBuilder, TableBuilder,
};
