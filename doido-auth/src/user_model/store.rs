//! Storage abstraction for [`super::UserModel`].

use crate::error::AuthError;
use crate::user_model::action::UserStoreAction;
use crate::user_model::plan::{FetchPlan, UserField};
use crate::user_model::row::UserRow;
use async_trait::async_trait;
use doido_model::sea_orm::{DatabaseConnection, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Engine-agnostic user persistence.
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn fetch_by_email(
        &self,
        email: &str,
        plan: &FetchPlan,
    ) -> Result<Option<UserRow>, AuthError>;

    async fn fetch_by_id(&self, id: Value, plan: &FetchPlan) -> Result<Option<UserRow>, AuthError>;

    /// Load a user by any unique column (e.g. confirmation / reset tokens).
    async fn fetch_by_field(
        &self,
        field: UserField,
        value: &str,
        plan: &FetchPlan,
    ) -> Result<Option<UserRow>, AuthError>;

    async fn apply(&self, action: UserStoreAction) -> Result<(), AuthError>;

    /// When the store is SQL-backed, returns the SeaORM connection (app registration only).
    fn sea_orm(&self) -> Option<&DatabaseConnection> {
        None
    }

    fn fetch_count(&self) -> usize {
        0
    }

    /// When the store is Redis, returns the concrete handle.
    #[cfg(feature = "auth-redis")]
    fn redis(&self) -> Option<&crate::user_model::redis_store::RedisUserStore> {
        None
    }
}

/// SeaORM / SQL user store (default).
pub struct SeaOrmUserStore {
    pub db: DatabaseConnection,
    fetches: AtomicUsize,
}

impl SeaOrmUserStore {
    pub fn new(db: DatabaseConnection) -> Arc<Self> {
        Arc::new(Self {
            db,
            fetches: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl UserStore for SeaOrmUserStore {
    async fn fetch_by_email(
        &self,
        email: &str,
        plan: &FetchPlan,
    ) -> Result<Option<UserRow>, AuthError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let backend = self.db.get_database_backend();
        crate::user_model::sql::sql_fetch_by_email(&self.db, backend, email, plan).await
    }

    async fn fetch_by_id(&self, id: Value, plan: &FetchPlan) -> Result<Option<UserRow>, AuthError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let backend = self.db.get_database_backend();
        crate::user_model::sql::sql_fetch_by_id(&self.db, backend, id, plan).await
    }

    async fn fetch_by_field(
        &self,
        field: UserField,
        value: &str,
        plan: &FetchPlan,
    ) -> Result<Option<UserRow>, AuthError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let backend = self.db.get_database_backend();
        crate::user_model::sql::sql_fetch_by_field(&self.db, backend, field, value, plan).await
    }

    async fn apply(&self, action: UserStoreAction) -> Result<(), AuthError> {
        let backend = self.db.get_database_backend();
        crate::user_model::sql::sql_apply_action(&self.db, backend, &action).await
    }

    fn sea_orm(&self) -> Option<&DatabaseConnection> {
        Some(&self.db)
    }

    fn fetch_count(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }
}

#[cfg(feature = "auth-redis")]
pub use crate::user_model::redis_store::RedisUserStore;

/// Build a store from [`crate::config::UserStoreConfig`] + db handle.
pub fn build_user_store(
    config: &crate::config::UserStoreConfig,
    db: DatabaseConnection,
) -> Result<Arc<dyn UserStore>, AuthError> {
    match config.store_type {
        crate::config::UserStoreType::SeaOrm => Ok(SeaOrmUserStore::new(db)),
        #[cfg(feature = "auth-redis")]
        crate::config::UserStoreType::Redis => {
            let store = RedisUserStore::connect(&config.endpoint, &config.key_prefix)?;
            Ok(store)
        }
        #[cfg(not(feature = "auth-redis"))]
        crate::config::UserStoreType::Redis => Err(AuthError::Config(
            "auth.user_store.type redis requires doido-auth feature `auth-redis`".into(),
        )),
    }
}
