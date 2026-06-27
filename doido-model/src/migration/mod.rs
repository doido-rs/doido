//! Rails-style schema migration helpers, grouped by the object they act on.
//!
//! Each object lives in its own file and is re-exported here, so operations are
//! reached directly — [`Table`], [`Column`], [`Index`], [`ForeignKey`] — and run
//! against any sea-orm [`ConnectionTrait`]:
//!
//! ```no_run
//! # async fn demo(db: &impl sea_orm::ConnectionTrait) -> Result<(), sea_orm::DbErr> {
//! use doido_model::migration::{Table, Column, Index, ForeignKey};
//!
//! Table::create(db, "users", |t| {
//!     t.string("email").not_null().unique_key();
//!     t.string("name");
//!     t.timestamps();
//! })
//! .await?;
//!
//! Column::add(db, "users", "age", |c| {
//!     c.integer();
//! })
//! .await?;
//!
//! Index::add(db, "users", &["email"]).await?;
//! ForeignKey::add(db, "posts", "user_id", "users", "id").await?;
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
pub use table::{Table, TableBuilder};
