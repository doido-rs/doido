//! Database seeds — run with `doido db seed` or
//! `cargo run --manifest-path db/seed/Cargo.toml`.
//!
//! Edit this file to insert fixture data using the models in `app/models/`.

#[allow(dead_code)]
#[path = "../../../app/models/mod.rs"]
mod models;

use doido::model::{config::YamlConfig, connect_with_url};

#[allow(unused_variables)]
async fn run_seed() -> doido::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .or_else(|_| YamlConfig::load().map(|c| c.database.url))
        .map_err(|e| doido::core::anyhow::anyhow!("{e}"))?;
    let db = connect_with_url(&url).await?;

    // Add seed data here using models from `app/models/`.
    //
    // Example (after `doido generate model User email:string:not_null`):
    //
    // use doido::model::sea_orm::{ActiveModelTrait, EntityTrait, Set};
    // use models::user::{ActiveModel, Entity};
    //
    // if Entity::find().one(&db).await?.is_none() {
    //     ActiveModel {
    //         email: Set("admin@example.com".into()),
    //         ..Default::default()
    //     }
    //     .insert(&db)
    //     .await?;
    // }
    // Seed an initial user so a fresh --auth app has a login out of the box.
    {
        use doido::model::QueryFilter;
        use doido::model::password::hash_password;
        use doido::model::sea_orm::{ColumnTrait, EntityTrait};
        use doido_auth::RegisterableAuthUser;
        use models::user::{Entity, Model};

        if Entity::find().one(&db).await?.is_none() {
            let digest = hash_password("password")?;
            Model::register(&db, "admin@example.com".into(), digest).await?;
            println!("seeded initial user: admin@example.com / password");
        }

        if Entity::find()
            .filter(models::user::Column::Email.eq("user@example.com"))
            .one(&db)
            .await?
            .is_none()
        {
            let digest = hash_password("password")?;
            Model::register(&db, "user@example.com".into(), digest).await?;
            println!("seeded demo user: user@example.com / password");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_seed().await {
        eprintln!("seed failed: {e}");
        std::process::exit(1);
    }
    println!("seed complete");
}
