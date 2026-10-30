//! Dashboard HTTP handlers.

use super::JobsWebState;
use crate::config::JobsConfig;
use crate::inspect::{
    decode_job_page_token, decode_queue_page_token, paginate_queue_names, JobBucket, JobPageAnchor,
    QueueCounts,
};
use crate::queue::JobQueue;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};

const PAGE_SIZE: u32 = 50;
const QUEUE_PAGE_SIZE: usize = 25;

#[derive(Debug, Deserialize, Default)]
pub struct DashboardQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub queue: Option<String>,
    /// Opaque cursor for job tables (replaces offset/limit pagination).
    #[serde(default)]
    pub page_token: Option<String>,
    /// Opaque cursor for the **Queues** tab.
    #[serde(default)]
    pub queue_page_token: Option<String>,
    /// Include pretty-printed payloads (`1`, `true`, `yes`, …).
    #[serde(default)]
    pub include_payload: Option<String>,
    #[serde(default)]
    pub flash: Option<String>,
}

fn tab_bucket(tab: &str) -> Option<JobBucket> {
    match tab {
        "in_queue" => Some(JobBucket::InQueue),
        "executing" => Some(JobBucket::Running),
        "retry" => Some(JobBucket::Retry),
        "dead" => Some(JobBucket::Dead),
        _ => None,
    }
}

fn active_tab(query: &DashboardQuery) -> &str {
    query.tab.as_deref().unwrap_or("queues")
}

fn job_row(job: &crate::queue::JobPayload, include_payload: bool) -> serde_json::Value {
    let payload = if include_payload {
        serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| "{}".into())
    } else {
        String::new()
    };
    json!({
        "id": job.id,
        "job_name": job.job_name,
        "queue": job.queue,
        "attempts": job.attempts,
        "max_retries": job.max_retries,
        "status": format!("{:?}", job.status),
        "run_at": job.run_at.to_rfc3339(),
        "error": job.error.as_deref().unwrap_or(""),
        "payload": payload,
    })
}

fn parse_job_page_token(token: Option<&str>) -> Option<JobPageAnchor> {
    token.and_then(decode_job_page_token)
}

async fn queue_counts_for_tab(
    state: &JobsWebState,
    tab: &str,
    queue_names: &[&str],
) -> Result<Vec<(String, QueueCounts)>, String> {
    if tab != "queues" {
        return Ok(Vec::new());
    }
    if let Some(ttl_secs) = state.config.webui.stats_ttl_secs {
        let ttl = Duration::from_secs(ttl_secs);
        let mut guard = state.stats_cache.lock().await;
        if let Some((fetched_at, ref data)) = *guard {
            if fetched_at.elapsed() < ttl {
                return Ok(data.clone());
            }
        }
        let counts = state
            .queue
            .queue_counts(queue_names)
            .await
            .map_err(|e| format!("{e:#}"))?;
        *guard = Some((Instant::now(), counts.clone()));
        return Ok(counts);
    }
    state
        .queue
        .queue_counts(queue_names)
        .await
        .map_err(|e| format!("{e:#}"))
}

pub async fn index(
    State(state): State<JobsWebState>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let tab = active_tab(&query);
    let include_payload = query
        .include_payload
        .as_deref()
        .is_some_and(|v| matches!(v, "1" | "true" | "yes" | "on"));
    let queues: Vec<String> = state.config.queues.clone();
    let queue_names: Vec<&str> = queues.iter().map(String::as_str).collect();
    let selected_queue = query
        .queue
        .clone()
        .filter(|q| queues.iter().any(|c| c == q))
        .or_else(|| queues.first().cloned());

    let counts = match queue_counts_for_tab(&state, tab, &queue_names).await {
        Ok(c) => c,
        Err(e) => {
            return Html(format!("Jobs dashboard error: {e}")).into_response();
        }
    };

    let queue_total = if tab == "queues" {
        counts.len()
    } else {
        queues.len()
    };
    let queue_page_anchor = query
        .queue_page_token
        .as_deref()
        .and_then(decode_queue_page_token);
    let queue_page = paginate_queue_names(
        counts,
        |(name, _)| name.as_str(),
        queue_page_anchor,
        QUEUE_PAGE_SIZE,
    );

    let job_page_anchor = parse_job_page_token(query.page_token.as_deref());
    let mut job_pagination = json!({
        "next_token": "",
        "prev_token": "",
        "has_next": false,
        "has_prev": false,
    });
    let mut jobs = Vec::new();
    let light_list = !include_payload;

    if tab != "queues" {
        if let Some(ref qname) = selected_queue {
            if let Some(bucket) = tab_bucket(tab) {
                match state
                    .queue
                    .list_jobs(qname, bucket, PAGE_SIZE, job_page_anchor, light_list)
                    .await
                {
                    Ok(page) => {
                        jobs = page.jobs;
                        let has_next = page.next_page_token.is_some();
                        let has_prev = page.prev_page_token.is_some();
                        job_pagination = json!({
                            "next_token": page.next_page_token.unwrap_or_default(),
                            "prev_token": page.prev_page_token.unwrap_or_default(),
                            "has_next": has_next,
                            "has_prev": has_prev,
                        });
                    }
                    Err(e) => {
                        return Html(format!("Jobs dashboard error: {e:#}")).into_response();
                    }
                }
            }
        }
    }

    let mount = state
        .config
        .webui
        .mount_path
        .trim_end_matches('/')
        .to_string();
    let ctx = json!({
        "mount": mount,
        "tab": tab,
        "flash": query.flash.as_deref().unwrap_or(""),
        "queues": queues,
        "selected_queue": selected_queue,
        "page_size": PAGE_SIZE,
        "include_payload": include_payload,
        "page_token": query.page_token.as_deref().unwrap_or(""),
        "queue_total": queue_total,
        "job_pagination": job_pagination,
        "queue_pagination": {
            "next_token": queue_page.next_page_token.clone().unwrap_or_default(),
            "prev_token": queue_page.prev_page_token.clone().unwrap_or_default(),
            "has_next": queue_page.next_page_token.is_some(),
            "has_prev": queue_page.prev_page_token.is_some(),
        },
        "queue_counts": queue_page.rows.iter().map(|(name, c)| queue_count_row(name, c)).collect::<Vec<_>>(),
        "jobs": jobs.iter().map(|j| job_row(j, include_payload)).collect::<Vec<_>>(),
        "show_dead_actions": tab == "dead",
        "now": Utc::now().to_rfc3339(),
    });

    match doido_view::render("jobs/dashboard", &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

fn queue_count_row(name: &str, c: &QueueCounts) -> serde_json::Value {
    json!({
        "name": name,
        "pending_ready": c.pending_ready,
        "scheduled": c.scheduled,
        "running": c.running,
        "retry": c.retry,
        "failed_ready": c.failed_ready,
        "dead": c.dead,
    })
}

fn redirect_dashboard(
    state: &JobsWebState,
    tab: &str,
    queue: Option<&str>,
    flash: &str,
) -> Redirect {
    let mount = state.config.webui.mount_path.trim_end_matches('/');
    let mut url = format!("{mount}?tab={tab}");
    if let Some(q) = queue {
        url.push_str(&format!("&queue={q}"));
    }
    if !flash.is_empty() {
        url.push_str(&format!("&flash={}", urlencoding_encode(flash)));
    }
    Redirect::to(&url)
}

fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "+".to_string(),
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' => c.to_string(),
            c => format!("%{:02X}", c as u8),
        })
        .collect()
}

pub async fn retry_one(
    State(state): State<JobsWebState>,
    Path((queue, job_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let _ = state.queue.retry_dead_job(&queue, &job_id).await;
    redirect_dashboard(&state, "dead", Some(&queue), "Job re-enqueued")
}

pub async fn discard_one(
    State(state): State<JobsWebState>,
    Path((queue, job_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let _ = state.queue.discard_dead_job(&queue, &job_id).await;
    redirect_dashboard(&state, "dead", Some(&queue), "Job discarded")
}

pub async fn retry_all(
    State(state): State<JobsWebState>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let queue = query.queue.as_deref().unwrap_or("default");
    retry_failed_queue(state.queue.as_ref(), &state.config, queue).await;
    redirect_dashboard(&state, "dead", Some(queue), "All dead jobs re-enqueued")
}

pub async fn discard_all(
    State(state): State<JobsWebState>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let queue = query.queue.as_deref().unwrap_or("default");
    let _ = state.queue.discard_dead(queue).await;
    redirect_dashboard(&state, "dead", Some(queue), "Dead queue cleared")
}

async fn retry_failed_queue(queue: &dyn JobQueue, config: &JobsConfig, q: &str) {
    let jobs = match queue.dead_jobs(q).await {
        Ok(j) => j,
        Err(e) => {
            doido_core::tracing::error!("webui retry_all: {e}");
            return;
        }
    };
    for job in jobs {
        let fresh = job.requeue_fresh();
        if let Err(e) = queue.enqueue(fresh).await {
            doido_core::tracing::error!("webui re-enqueue: {e}");
        }
    }
    if config.queues.iter().any(|name| name == q) {
        if let Err(e) = queue.discard_dead(q).await {
            doido_core::tracing::error!("webui clear dead: {e}");
        }
    }
}
