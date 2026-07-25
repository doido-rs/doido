//! Database-backed pub/sub (feature `cable-db`): a durable `cable_messages` log
//! that publishers append to and subscribers poll, so broadcasts survive across
//! processes/restarts. `messages_since` is the tested durable core; `subscribe`
//! also fans out to in-process subscribers immediately.

use crate::pubsub::PubSub;
use doido_core::Result;
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};

const CAPACITY: usize = 128;

/// A [`PubSub`] backed by a `cable_messages` table.
pub struct DbPubSub {
    conn: DatabaseConnection,
    senders: Mutex<HashMap<String, broadcast::Sender<String>>>,
}

impl DbPubSub {
    /// Connect and ensure the `cable_messages` table exists.
    pub async fn connect(conn: DatabaseConnection) -> Result<Self> {
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS cable_messages (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                stream TEXT NOT NULL, \
                payload TEXT NOT NULL)",
        )
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("cable_messages migrate failed: {e}"))?;
        Ok(Self {
            conn,
            senders: Mutex::new(HashMap::new()),
        })
    }

    async fn insert(&self, stream: &str, payload: &str) -> Result<()> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO cable_messages (stream, payload) VALUES (?, ?)",
            [Value::from(stream), Value::from(payload)],
        );
        self.conn
            .execute_raw(stmt)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("cable insert failed: {e}"))?;
        Ok(())
    }

    /// Messages for `stream` with id greater than `after_id`, oldest first.
    pub async fn messages_since(&self, stream: &str, after_id: i64) -> Result<Vec<(i64, String)>> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, payload FROM cable_messages WHERE stream = ? AND id > ? ORDER BY id",
            [Value::from(stream), Value::from(after_id)],
        );
        let rows = self
            .conn
            .query_all_raw(stmt)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("cable query failed: {e}"))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row
                .try_get("", "id")
                .map_err(|e| doido_core::anyhow::anyhow!("cable row id: {e}"))?;
            let payload: String = row
                .try_get("", "payload")
                .map_err(|e| doido_core::anyhow::anyhow!("cable row payload: {e}"))?;
            out.push((id, payload));
        }
        Ok(out)
    }
}

#[async_trait::async_trait]
impl PubSub for DbPubSub {
    async fn subscribe(&self, stream: &str) -> Result<broadcast::Receiver<String>> {
        let mut senders = self.senders.lock().await;
        if let Some(sender) = senders.get(stream) {
            return Ok(sender.subscribe());
        }
        let (tx, rx) = broadcast::channel(CAPACITY);
        senders.insert(stream.to_string(), tx);
        Ok(rx)
    }

    async fn publish(&self, stream: &str, message: &str) -> Result<()> {
        self.insert(stream, message).await?;
        if let Some(sender) = self.senders.lock().await.get(stream) {
            let _ = sender.send(message.to_string());
        }
        Ok(())
    }

    async fn unsubscribe(&self, stream: &str) -> Result<()> {
        self.senders.lock().await.remove(stream);
        Ok(())
    }
}
