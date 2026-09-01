//! Database seeds — run with `doido db seed`.
//!
//! Edit this file to insert fixture data using the models in `app/models/`. It
//! runs in-process from the app binary, so its `INSERT`s are logged.

use doido::model::sea_orm::DatabaseConnection;

pub async fn run(db: &DatabaseConnection) -> doido::Result<()> {
    // Seed an initial user so a fresh --auth app has a login out of the box.
    {
        use crate::models::user::{Entity, Model};
        use doido::model::password::hash_password;
        use doido::model::sea_orm::{ColumnTrait, EntityTrait};
        use doido::model::QueryFilter;
        use doido_auth::RegisterableAuthUser;

        if Entity::find().one(db).await?.is_none() {
            let digest = hash_password("password")?;
            Model::register(db, "admin@example.com".into(), digest).await?;
            println!("seeded initial user: admin@example.com / password");
        }

        if Entity::find()
            .filter(crate::models::user::Column::Email.eq("user@example.com"))
            .one(db)
            .await?
            .is_none()
        {
            let digest = hash_password("password")?;
            Model::register(db, "user@example.com".into(), digest).await?;
            println!("seeded demo user: user@example.com / password");
        }
    }
    Ok(())
}
