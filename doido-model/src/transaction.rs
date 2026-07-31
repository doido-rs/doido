//! A transaction convenience wrapper (Rails `Model.transaction do … end`).
//!
//! [`transaction`] begins a database transaction, runs the supplied async work,
//! commits when it returns `Ok`, and rolls back on `Err` (or panic-free early
//! return), surfacing errors as [`doido_core::Result`].

use crate::sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};
use doido_core::Result;

/// Run `work` inside a transaction: commit on `Ok`, roll back on `Err`.
pub async fn transaction<T>(
    conn: &DatabaseConnection,
    work: impl AsyncFnOnce(&DatabaseTransaction) -> Result<T>,
) -> Result<T> {
    let txn = conn
        .begin()
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("begin transaction failed: {e}"))?;

    match work(&txn).await {
        Ok(value) => {
            txn.commit()
                .await
                .map_err(|e| doido_core::anyhow::anyhow!("commit failed: {e}"))?;
            Ok(value)
        }
        Err(err) => {
            let _ = txn.rollback().await;
            Err(err)
        }
    }
}
