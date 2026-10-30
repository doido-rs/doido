//! Aggregated dashboard stats (single query per `queue_counts` call).

use crate::inspect::QueueCounts;
use doido_core::{anyhow::anyhow, Result};
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, Statement, Value};
use std::collections::HashMap;

const STATUS_PENDING: &str = "pending";
const STATUS_RUNNING: &str = "running";
const STATUS_FAILED: &str = "failed";
const STATUS_DEAD: &str = "dead";

/// One grouped SQL query for all configured queues.
pub async fn aggregate_queue_counts(
    conn: &DatabaseConnection,
    queues: &[&str],
    now_ms: i64,
) -> Result<Vec<(String, QueueCounts)>> {
    if queues.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<String> = queues
        .iter()
        .enumerate()
        .map(|(i, _)| format!("${}", i + 2))
        .collect();
    let in_list = placeholders.join(", ");
    let sql = format!(
        r#"
        SELECT
            queue,
            COALESCE(SUM(CASE WHEN status = '{pending}' AND run_at <= $1 THEN 1 ELSE 0 END), 0) AS pending_ready,
            COALESCE(SUM(CASE WHEN status = '{pending}' AND run_at > $1 THEN 1 ELSE 0 END), 0) AS scheduled,
            COALESCE(SUM(CASE WHEN status = '{running}' THEN 1 ELSE 0 END), 0) AS running,
            COALESCE(SUM(CASE WHEN status = '{failed}' AND run_at > $1 THEN 1 ELSE 0 END), 0) AS retry,
            COALESCE(SUM(CASE WHEN status = '{failed}' AND run_at <= $1 THEN 1 ELSE 0 END), 0) AS failed_ready,
            COALESCE(SUM(CASE WHEN status = '{dead}' THEN 1 ELSE 0 END), 0) AS dead
        FROM doido_jobs
        WHERE queue IN ({in_list})
        GROUP BY queue
        "#,
        pending = STATUS_PENDING,
        running = STATUS_RUNNING,
        failed = STATUS_FAILED,
        dead = STATUS_DEAD,
        in_list = in_list,
    );

    // SQLite tests use `?` placeholders; detect via backend.
    let backend = conn.get_database_backend();
    let (sql, values) = match backend {
        doido_model::sea_orm::DatabaseBackend::Sqlite => {
            let qmarks: Vec<String> = (0..queues.len()).map(|_| "?".to_string()).collect();
            let sqlite_sql = format!(
                r#"
                SELECT
                    queue,
                    SUM(CASE WHEN status = '{pending}' AND run_at <= ? THEN 1 ELSE 0 END) AS pending_ready,
                    SUM(CASE WHEN status = '{pending}' AND run_at > ? THEN 1 ELSE 0 END) AS scheduled,
                    SUM(CASE WHEN status = '{running}' THEN 1 ELSE 0 END) AS running,
                    SUM(CASE WHEN status = '{failed}' AND run_at > ? THEN 1 ELSE 0 END) AS retry,
                    SUM(CASE WHEN status = '{failed}' AND run_at <= ? THEN 1 ELSE 0 END) AS failed_ready,
                    SUM(CASE WHEN status = '{dead}' THEN 1 ELSE 0 END) AS dead
                FROM doido_jobs
                WHERE queue IN ({in_list})
                GROUP BY queue
                "#,
                pending = STATUS_PENDING,
                running = STATUS_RUNNING,
                failed = STATUS_FAILED,
                dead = STATUS_DEAD,
                in_list = qmarks.join(", "),
            );
            let mut vals: Vec<Value> = vec![now_ms.into(); 4];
            for q in queues {
                vals.push(q.to_string().into());
            }
            (sqlite_sql, vals)
        }
        doido_model::sea_orm::DatabaseBackend::MySql => {
            let qmarks: Vec<String> = (0..queues.len()).map(|_| "?".to_string()).collect();
            let mysql_sql = format!(
                r#"
                SELECT
                    queue,
                    COALESCE(SUM(CASE WHEN status = '{pending}' AND run_at <= ? THEN 1 ELSE 0 END), 0) AS pending_ready,
                    COALESCE(SUM(CASE WHEN status = '{pending}' AND run_at > ? THEN 1 ELSE 0 END), 0) AS scheduled,
                    COALESCE(SUM(CASE WHEN status = '{running}' THEN 1 ELSE 0 END), 0) AS running,
                    COALESCE(SUM(CASE WHEN status = '{failed}' AND run_at > ? THEN 1 ELSE 0 END), 0) AS retry,
                    COALESCE(SUM(CASE WHEN status = '{failed}' AND run_at <= ? THEN 1 ELSE 0 END), 0) AS failed_ready,
                    COALESCE(SUM(CASE WHEN status = '{dead}' THEN 1 ELSE 0 END), 0) AS dead
                FROM doido_jobs
                WHERE queue IN ({in_list})
                GROUP BY queue
                "#,
                pending = STATUS_PENDING,
                running = STATUS_RUNNING,
                failed = STATUS_FAILED,
                dead = STATUS_DEAD,
                in_list = qmarks.join(", "),
            );
            let mut vals: Vec<Value> = vec![now_ms.into(); 4];
            for q in queues {
                vals.push(q.to_string().into());
            }
            (mysql_sql, vals)
        }
        _ => {
            let mut vals: Vec<Value> = vec![now_ms.into()];
            for q in queues {
                vals.push(q.to_string().into());
            }
            (sql, vals)
        }
    };

    let rows = conn
        .query_all_raw(Statement::from_sql_and_values(backend, &sql, values))
        .await
        .map_err(|e| anyhow!("aggregate queue_counts failed: {e}"))?;

    let mut by_name: HashMap<String, QueueCounts> = HashMap::new();
    for row in rows {
        let queue: String = row.try_get("", "queue").map_err(|e| anyhow!("{e}"))?;
        let pending_ready: i64 = row
            .try_get("", "pending_ready")
            .map_err(|e| anyhow!("{e}"))?;
        let scheduled: i64 = row.try_get("", "scheduled").map_err(|e| anyhow!("{e}"))?;
        let running: i64 = row.try_get("", "running").map_err(|e| anyhow!("{e}"))?;
        let retry: i64 = row.try_get("", "retry").map_err(|e| anyhow!("{e}"))?;
        let failed_ready: i64 = row
            .try_get("", "failed_ready")
            .map_err(|e| anyhow!("{e}"))?;
        let dead: i64 = row.try_get("", "dead").map_err(|e| anyhow!("{e}"))?;
        by_name.insert(
            queue,
            QueueCounts {
                pending_ready: pending_ready as u32,
                scheduled: scheduled as u32,
                running: running as u32,
                retry: retry as u32,
                failed_ready: failed_ready as u32,
                dead: dead as u32,
            },
        );
    }

    Ok(queues
        .iter()
        .map(|name| {
            let counts = by_name.remove(*name).unwrap_or_default();
            ((*name).to_string(), counts)
        })
        .collect())
}
