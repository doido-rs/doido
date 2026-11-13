use crate::error::AuthError;
use crate::user_model::action::{FieldChange, UserLocator, UserStoreAction};
use crate::user_model::plan::{FetchPlan, UserField};
use crate::user_model::row::UserRow;
use crate::user_model::store::UserStore;
use async_trait::async_trait;
use doido_model::sea_orm::Value;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct RedisUserStore {
    client: redis::Client,
    prefix: String,
    fetches: AtomicUsize,
}

impl RedisUserStore {
    pub fn connect(endpoint: &str, prefix: &str) -> Result<Arc<dyn UserStore>, AuthError> {
        let client = redis::Client::open(endpoint)
            .map_err(|e| AuthError::Config(format!("redis connect: {e}")))?;
        Ok(Arc::new(Self {
            client,
            prefix: prefix.trim_matches('/').to_string(),
            fetches: AtomicUsize::new(0),
        }))
    }

    fn email_key(&self, email: &str) -> String {
        format!("{}:user:email:{}", self.prefix, email)
    }

    fn id_key(&self, id: &str) -> String {
        format!("{}:user:id:{}", self.prefix, id)
    }

    fn lookup_key(&self, field: UserField, token: &str) -> String {
        let kind = match field {
            UserField::ConfirmationToken => "confirmation_token",
            UserField::ResetPasswordToken => "reset_password_token",
            other => other.column_name(),
        };
        format!("{}:lookup:{kind}:{token}", self.prefix)
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, AuthError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))
    }

    async fn hash_for_email(
        &self,
        conn: &mut redis::aio::MultiplexedConnection,
        email: &str,
    ) -> Result<HashMap<String, String>, AuthError> {
        let key = self.email_key(email);
        conn.hgetall(&key)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))
    }
}

fn row_from_hash(fields: &[(String, String)], plan: &FetchPlan) -> UserRow {
    let mut map = HashMap::<&str, String>::new();
    for (k, v) in fields {
        map.insert(k.as_str(), v.clone());
    }
    let mut row = UserRow::default();
    for field in &plan.fields {
        let col = field.column_name();
        let value = map.get(col).cloned();
        if field == &UserField::Id {
            if let Some(s) = value {
                if let Ok(n) = s.parse::<i64>() {
                    row.set_id_json(serde_json::json!(n));
                } else {
                    row.set_id_json(serde_json::Value::String(s));
                }
            }
        } else if field == &UserField::FailedAttempts || field == &UserField::SignInCount {
            row.set_from_column(*field, value);
        } else {
            row.set_from_column(*field, value);
        }
    }
    row
}

fn row_from_map(map: HashMap<String, String>, plan: &FetchPlan) -> UserRow {
    let fields: Vec<(String, String)> = map.into_iter().collect();
    row_from_hash(&fields, plan)
}

#[async_trait]
impl UserStore for RedisUserStore {
    async fn fetch_by_email(
        &self,
        email: &str,
        plan: &FetchPlan,
    ) -> Result<Option<UserRow>, AuthError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let mut conn = self.connection().await?;
        let map = self.hash_for_email(&mut conn, email).await?;
        if map.is_empty() {
            return Ok(None);
        }
        Ok(Some(row_from_map(map, plan)))
    }

    async fn fetch_by_id(&self, id: Value, plan: &FetchPlan) -> Result<Option<UserRow>, AuthError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let id_str = match id {
            Value::BigInt(Some(i)) => i.to_string(),
            Value::BigUnsigned(Some(u)) => u.to_string(),
            Value::String(Some(s)) => s,
            other => other.to_string(),
        };
        let mut conn = self.connection().await?;
        let email: Option<String> = conn
            .get(self.id_key(&id_str))
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let email = match email {
            Some(e) => e,
            None => return Ok(None),
        };
        self.fetch_by_email(&email, plan).await
    }

    async fn fetch_by_field(
        &self,
        field: UserField,
        value: &str,
        plan: &FetchPlan,
    ) -> Result<Option<UserRow>, AuthError> {
        if field == UserField::Email {
            return self.fetch_by_email(value, plan).await;
        }
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let mut conn = self.connection().await?;
        let email: Option<String> = conn
            .get(self.lookup_key(field, value))
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let email = match email {
            Some(e) => e,
            None => return Ok(None),
        };
        self.fetch_by_email(&email, plan).await
    }

    async fn apply(&self, action: UserStoreAction) -> Result<(), AuthError> {
        let mut conn = self.connection().await?;
        match action {
            UserStoreAction::Patch {
                locator,
                changes,
                add_indexes,
                remove_indexes,
            } => {
                self.apply_patch(&mut conn, &locator, &changes, &add_indexes, &remove_indexes)
                    .await?;
            }
            UserStoreAction::RecordSuccessfulSignIn {
                email,
                signed_in_at,
                ip,
            } => {
                let key = self.email_key(&email);
                let map = self.hash_for_email(&mut conn, &email).await?;
                let count: i32 = map
                    .get("sign_in_count")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
                    + 1;
                let prev_at = map.get("current_sign_in_at").cloned();
                let prev_ip = map.get("current_sign_in_ip").cloned();
                let ip_val = ip.unwrap_or_default();
                let mut pipe = redis::pipe();
                pipe.atomic()
                    .hset(&key, "sign_in_count", count.to_string())
                    .hset(&key, "current_sign_in_at", signed_in_at.as_str());
                if let Some(prev) = prev_at {
                    pipe.hset(&key, "last_sign_in_at", prev);
                }
                pipe.hset(&key, "current_sign_in_ip", ip_val);
                if let Some(prev) = prev_ip {
                    pipe.hset(&key, "last_sign_in_ip", prev);
                }
                let _: () = pipe
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn fetch_count(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }

    fn redis(&self) -> Option<&RedisUserStore> {
        Some(self)
    }
}

impl RedisUserStore {
    async fn email_for_locator(
        &self,
        conn: &mut redis::aio::MultiplexedConnection,
        locator: &UserLocator,
    ) -> Result<Option<String>, AuthError> {
        match locator {
            UserLocator::ByEmail(email) => Ok(Some(email.clone())),
            UserLocator::ByField { field, value } if *field == UserField::Email => {
                Ok(Some(value.clone()))
            }
            UserLocator::ByField { field, value } => conn
                .get(self.lookup_key(*field, value))
                .await
                .map_err(|e| AuthError::Internal(e.to_string())),
        }
    }

    async fn apply_patch(
        &self,
        conn: &mut redis::aio::MultiplexedConnection,
        locator: &UserLocator,
        changes: &[FieldChange],
        add_indexes: &[crate::user_model::action::TokenIndex],
        remove_indexes: &[crate::user_model::action::TokenIndex],
    ) -> Result<(), AuthError> {
        let email = match self.email_for_locator(conn, locator).await? {
            Some(e) => e,
            None => return Ok(()),
        };
        let key = self.email_key(&email);
        let mut pipe = redis::pipe();
        pipe.atomic();
        for change in changes {
            match change {
                FieldChange::Set { field, value } => {
                    pipe.hset(&key, field.column_name(), value.as_str());
                }
                FieldChange::Clear { field } => {
                    pipe.hdel(&key, field.column_name());
                }
                FieldChange::Increment { field, by } => {
                    pipe.hincr(&key, field.column_name(), *by as i64);
                }
            }
        }
        for idx in add_indexes {
            let lookup = self.lookup_key(idx.field, &idx.token);
            pipe.set(&lookup, email.as_str());
        }
        for idx in remove_indexes {
            let lookup = self.lookup_key(idx.field, &idx.token);
            pipe.del(&lookup);
        }
        let _: () = pipe
            .query_async(conn)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Insert or replace a user hash + id index (sign-up).
    pub async fn insert_user(
        &self,
        id: i64,
        email: &str,
        password_digest: &str,
    ) -> Result<(), AuthError> {
        let mut conn = self.connection().await?;
        let email_key = self.email_key(email);
        let _: () = redis::pipe()
            .atomic()
            .hset_multiple(
                &email_key,
                &[
                    ("id", id.to_string()),
                    ("email", email.to_string()),
                    ("password_digest", password_digest.to_string()),
                ],
            )
            .set(self.id_key(&id.to_string()), email)
            .query_async(&mut conn)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn hset_fields(
        &self,
        email: &str,
        fields: &[(&str, String)],
    ) -> Result<(), AuthError> {
        let mut conn = self.connection().await?;
        let key = self.email_key(email);
        conn.hset_multiple::<_, _, _, ()>(&key, fields)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(())
    }
}
