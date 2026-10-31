//! Enqueue form: pick a registered job and run or schedule it.

use super::dashboard::redirect_dashboard;
use super::JobsWebState;
use crate::context::job_context_from_runtime;
use crate::perform::{enqueue_jobs, enqueue_jobs_at, perform_jobs_now};
use crate::registry::{
    build_job_payloads, parse_job_payloads_json, suggest_job_names, JobRegistry,
};
use axum::{
    extract::{Form, State},
    response::IntoResponse,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use doido_core::{anyhow::anyhow, Result};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct EnqueueForm {
    pub job_name: String,
    #[serde(default)]
    pub payload: String,
    #[serde(default)]
    pub queue: String,
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub max_retries: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub run_at: String,
}

pub async fn submit(
    State(state): State<JobsWebState>,
    Form(form): Form<EnqueueForm>,
) -> impl IntoResponse {
    match submit_inner(&state, form).await {
        Ok(flash) => redirect_dashboard(&state, "enqueue", None, &flash),
        Err(e) => redirect_dashboard(&state, "enqueue", None, &format!("Error: {e:#}")),
    }
}

async fn submit_inner(state: &JobsWebState, form: EnqueueForm) -> Result<String> {
    let registry = JobRegistry::from_inventory();
    let job_name = form.job_name.trim();
    if job_name.is_empty() {
        return Err(anyhow!("job name is required"));
    }

    if registry.defaults_for(job_name).is_none() {
        let known = registry.names();
        let hints = suggest_job_names(&known, job_name, 5);
        let mut msg = format!("unknown job `{job_name}`");
        if !hints.is_empty() {
            msg.push_str(" (did you mean: ");
            msg.push_str(&hints.join(", "));
            msg.push(')');
        }
        return Err(anyhow!(msg));
    }

    let payloads = parse_job_payloads_json(Some(form.payload.as_str()))?;
    let queue = optional_non_empty(form.queue);
    let timeout = parse_optional_u64(form.timeout.as_str())?;
    let max_retries = parse_optional_u32(form.max_retries.as_str())?;
    let queue_name = queue.clone().unwrap_or_else(|| {
        registry
            .defaults_for(job_name)
            .map(|d| d.queue.to_string())
            .unwrap_or_else(|| "default".to_string())
    });

    let jobs = build_job_payloads(&registry, job_name, payloads, queue, timeout, max_retries)?;
    let n = jobs.len();

    match form.mode.as_str() {
        "enqueue_now" => {
            let ids = enqueue_jobs(state.queue.as_ref(), jobs).await?;
            Ok(flash_enqueued(n, &ids, &queue_name, job_name))
        }
        "enqueue_at" => {
            let at = parse_run_at_utc(form.run_at.as_str())?;
            let ids = enqueue_jobs_at(state.queue.as_ref(), jobs, at).await?;
            Ok(flash_scheduled(
                n,
                &ids,
                &queue_name,
                job_name,
                &at.to_rfc3339(),
            ))
        }
        "perform_now" => {
            let ctx = Arc::new(job_context_from_runtime());
            perform_jobs_now(&registry, jobs, ctx).await?;
            Ok(if n == 1 {
                format!("Performed job_name={job_name} in this server process")
            } else {
                format!("Performed {n} job(s) job_name={job_name} in this server process")
            })
        }
        other => Err(anyhow!(
            "invalid mode `{other}` (expected enqueue_now, enqueue_at, or perform_now)"
        )),
    }
}

fn flash_enqueued(n: usize, ids: &[crate::queue::JobId], queue: &str, job_name: &str) -> String {
    if n == 1 {
        format!(
            "Enqueued job_id={} queue={queue} job_name={job_name}",
            ids[0]
        )
    } else {
        format!(
            "Enqueued {n} job(s) queue={queue} job_name={job_name} ids={}",
            ids.join(", ")
        )
    }
}

fn flash_scheduled(
    n: usize,
    ids: &[crate::queue::JobId],
    queue: &str,
    job_name: &str,
    run_at: &str,
) -> String {
    if n == 1 {
        format!(
            "Scheduled job_id={} queue={queue} job_name={job_name} run_at={run_at}",
            ids[0]
        )
    } else {
        format!(
            "Scheduled {n} job(s) queue={queue} job_name={job_name} run_at={run_at} ids={}",
            ids.join(", ")
        )
    }
}

fn optional_non_empty(s: String) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn parse_optional_u64(s: &str) -> Result<Option<u64>> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse()
        .map(Some)
        .map_err(|_| anyhow!("timeout must be a non-negative integer"))
}

fn parse_optional_u32(s: &str) -> Result<Option<u32>> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse()
        .map(Some)
        .map_err(|_| anyhow!("max_retries must be a non-negative integer"))
}

/// `datetime-local` value interpreted as UTC (`YYYY-MM-DDTHH:MM` or with seconds).
fn parse_run_at_utc(raw: &str) -> Result<DateTime<Utc>> {
    let t = raw.trim();
    if t.is_empty() {
        return Err(anyhow!("run_at is required when scheduling"));
    }
    let naive = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| anyhow!("run_at must be a valid datetime (UTC)"))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_run_at_utc_accepts_datetime_local() {
        let at = parse_run_at_utc("2026-12-01T15:30").unwrap();
        assert_eq!(at.to_rfc3339(), "2026-12-01T15:30:00+00:00");
    }
}
