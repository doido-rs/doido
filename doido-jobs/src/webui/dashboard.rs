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

fn job_row(job: &crate::queue::JobPayload) -> serde_json::Value {
    json!({
        "id": job.id,
        "job_name": job.job_name,
        "queue": job.queue,
        "attempts": job.attempts,
        "max_retries": job.max_retries,
        "status": format!("{:?}", job.status),
        "run_at": job.run_at.to_rfc3339(),
        "error": job.error.as_deref().unwrap_or(""),
        "payload": serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| "{}".into()),
    })
}

fn parse_job_page_token(token: Option<&str>) -> Option<JobPageAnchor> {
    token.and_then(decode_job_page_token)
}

pub async fn index(
    State(state): State<JobsWebState>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let tab = active_tab(&query);
    let queues: Vec<String> = state.config.queues.clone();
    let queue_names: Vec<&str> = queues.iter().map(String::as_str).collect();
    let selected_queue = query
        .queue
        .clone()
        .filter(|q| queues.iter().any(|c| c == q))
        .or_else(|| queues.first().cloned());

    let counts = match state.queue.queue_counts(&queue_names).await {
        Ok(c) => c,
        Err(e) => {
            return Html(format!("Jobs dashboard error: {e:#}")).into_response();
        }
    };

    let queue_total = counts.len();
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

    if tab != "queues" {
        if let Some(ref qname) = selected_queue {
            if let Some(bucket) = tab_bucket(tab) {
                match state
                    .queue
                    .list_jobs(qname, bucket, PAGE_SIZE, job_page_anchor)
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
        "queue_total": queue_total,
        "job_pagination": job_pagination,
        "queue_pagination": {
            "next_token": queue_page.next_page_token.clone().unwrap_or_default(),
            "prev_token": queue_page.prev_page_token.clone().unwrap_or_default(),
            "has_next": queue_page.next_page_token.is_some(),
            "has_prev": queue_page.prev_page_token.is_some(),
        },
        "queue_counts": queue_page.rows.iter().map(|(name, c)| queue_count_row(name, c)).collect::<Vec<_>>(),
        "jobs": jobs.iter().map(job_row).collect::<Vec<_>>(),
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
