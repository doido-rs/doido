//! Database seeds — run with `doido db seed`.
//!
//! Edit this file to insert fixture data using the models in `app/models/`. It
//! runs in-process from the app binary (registered via `.seeder` in
//! `src/main.rs`), so its `INSERT`s are logged like any other statement.

use doido::model::sea_orm::DatabaseConnection;

#[allow(unused_variables)]
pub async fn run(db: &DatabaseConnection) -> doido::Result<()> {
    // Add seed data here using models from `app/models/`.
    //
    // Example (after `doido generate model User email:string:not_null`):
    //
    // use doido::model::sea_orm::{ActiveModelTrait, EntityTrait, Set};
    // use crate::models::user::{ActiveModel, Entity};
    //
    // if Entity::find().one(db).await?.is_none() {
    //     ActiveModel {
    //         email: Set("admin@example.com".into()),
    //         ..Default::default()
    //     }
    //     .insert(db)
    //     .await?;
    // }
{doido_auth_seed}
    Ok(())
}
