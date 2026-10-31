//! Bulk actions on job listing tabs (POST form).

use super::dashboard::{
    redirect_dashboard_view, retry_failed_queue, DashboardViewQuery, PAGE_SIZE,
};
use super::JobsWebState;
use crate::queue::JobQueue;
use axum::{
    extract::{RawForm, State},
    response::IntoResponse,
};
use doido_core::{anyhow::anyhow, Result};

#[derive(Debug, Default)]
struct BulkForm {
    action: String,
    tab: String,
    queue: String,
    job_ids: Vec<String>,
    page_token: String,
    include_payload: String,
}

fn parse_bulk_form(raw: &RawForm) -> BulkForm {
    let mut form = BulkForm::default();
    for (key, value) in form_urlencoded::parse(raw.0.as_ref()) {
        match key.as_ref() {
            "action" => form.action = value.into_owned(),
            "tab" => form.tab = value.into_owned(),
            "queue" => form.queue = value.into_owned(),
            "job_ids" => form.job_ids.push(value.into_owned()),
            "page_token" => form.page_token = value.into_owned(),
            "include_payload" => form.include_payload = value.into_owned(),
            _ => {}
        }
    }
    form
}

fn view_from_form(form: &BulkForm) -> DashboardViewQuery {
    DashboardViewQuery {
        page_token: if form.page_token.is_empty() {
            None
        } else {
            Some(form.page_token.clone())
        },
        include_payload: matches!(form.include_payload.as_str(), "1" | "true" | "yes" | "on"),
    }
}

fn allowed_for_tab(tab: &str, action: &str) -> bool {
    match tab {
        "in_queue" | "executing" | "retry" => action == "delete_selected",
        "dead" => matches!(
            action,
            "retry_selected" | "discard_selected" | "retry_all" | "discard_all"
        ),
        _ => false,
    }
}

pub async fn submit(State(state): State<JobsWebState>, raw: RawForm) -> impl IntoResponse {
    let form = parse_bulk_form(&raw);
    let view = view_from_form(&form);
    let tab = form.tab.as_str();
    let queue = form.queue.as_str();
    let redirect = |flash: &str| redirect_dashboard_view(&state, tab, Some(queue), flash, &view);

    if !state.config.queues.iter().any(|q| q == queue) {
        return redirect("Error: unknown queue");
    }
    if !allowed_for_tab(tab, form.action.as_str()) {
        return redirect("Error: action not allowed for this tab");
    }

    let job_ids: Vec<&str> = form
        .job_ids
        .iter()
        .map(String::as_str)
        .take(PAGE_SIZE as usize)
        .collect();

    match form.action.as_str() {
        "delete_selected" => match delete_selected(state.queue.as_ref(), queue, &job_ids).await {
            Ok(n) => redirect(&format!("Removed {n} job(s)")),
            Err(e) => redirect(&format!("Error: {e:#}")),
        },
        "retry_selected" => match retry_selected(state.queue.as_ref(), queue, &job_ids).await {
            Ok(n) => redirect(&format!("Re-enqueued {n} job(s)")),
            Err(e) => redirect(&format!("Error: {e:#}")),
        },
        "discard_selected" => match discard_selected(state.queue.as_ref(), queue, &job_ids).await {
            Ok(n) => redirect(&format!("Discarded {n} job(s)")),
            Err(e) => redirect(&format!("Error: {e:#}")),
        },
        "retry_all" => {
            retry_failed_queue(state.queue.as_ref(), &state.config, queue).await;
            redirect("All dead jobs re-enqueued")
        }
        "discard_all" => match state.queue.discard_dead(queue).await {
            Ok(n) => redirect(&format!("Discarded {n} dead job(s)")),
            Err(e) => redirect(&format!("Error: {e:#}")),
        },
        _ => redirect("Error: unknown action"),
    }
}

async fn delete_selected(queue: &dyn JobQueue, q: &str, job_ids: &[&str]) -> Result<u32> {
    if job_ids.is_empty() {
        return Err(anyhow!("select at least one job"));
    }
    let mut n = 0u32;
    for id in job_ids {
        if queue.cancel_job(q, id).await? {
            n += 1;
        }
    }
    Ok(n)
}

async fn retry_selected(queue: &dyn JobQueue, q: &str, job_ids: &[&str]) -> Result<u32> {
    if job_ids.is_empty() {
        return Err(anyhow!("select at least one job"));
    }
    let mut n = 0u32;
    for id in job_ids {
        let before = queue.dead_jobs(q).await?.iter().any(|j| j.id == *id);
        if before {
            queue.retry_dead_job(q, id).await?;
            n += 1;
        }
    }
    Ok(n)
}

async fn discard_selected(queue: &dyn JobQueue, q: &str, job_ids: &[&str]) -> Result<u32> {
    if job_ids.is_empty() {
        return Err(anyhow!("select at least one job"));
    }
    let mut n = 0u32;
    for id in job_ids {
        if queue.discard_dead_job(q, id).await? {
            n += 1;
        }
    }
    Ok(n)
}
