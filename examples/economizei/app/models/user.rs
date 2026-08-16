//! User model extensions — implements [`AuthUser`] for session and credential auth.
//! Safe to edit; never overwritten by generators.
#![allow(dead_code, unused_imports)]

pub use super::_entities::users::*;

use doido::model::sea_orm::ActiveModelBehavior;

impl ActiveModelBehavior for ActiveModel {}

use doido::model::password::HasSecurePassword;
use doido::model::sea_orm::entity::prelude::*;
use doido_auth::AuthUser;

impl HasSecurePassword for Model {
    fn password_digest(&self) -> &str {
        &self.password_digest
    }
}

impl AuthUser for Model {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn password_digest(&self) -> Option<&str> {
        Some(&self.password_digest)
    }

    async fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> doido::Result<Option<Self>> {
        Entity::find()
            .filter(Column::Email.eq(email))
            .one(db)
            .await
            .map_err(Into::into)
    }

    async fn find_by_id(
        db: &DatabaseConnection,
        id: Self::Id,
    ) -> doido::Result<Option<Self>> {
        Entity::find_by_id(id).one(db).await.map_err(Into::into)
    }
}

impl Model {
    pub async fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> doido::Result<Option<Self>> {
        <Self as AuthUser>::find_by_email(db, email).await
    }
}
