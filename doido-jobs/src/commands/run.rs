//! `doido jobs run` / `perform` — in-process perform or enqueue for the worker.

use crate::context::job_context_from_runtime;
use crate::perform::{enqueue_jobs, perform_jobs_now};
use crate::registry::{
    build_job_payloads, parse_job_payloads_json, suggest_job_names, JobRegistry,
};
use crate::JobQueue;
use clap::Args;
use doido_core::{anyhow::anyhow, Result};
use std::sync::Arc;

/// Arguments for `doido jobs run <job_name>`.
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Registered job name (the `#[job]` function name).
    pub job_name: String,

    /// JSON object deserialized into the handler's payload type (default `{}`).
    #[arg(long)]
    pub payload: Option<String>,

    /// Queue name (defaults to the `#[job]` macro's `queue` attribute).
    #[arg(long)]
    pub queue: Option<String>,

    /// Enqueue for the worker instead of running in this process.
    #[arg(long)]
    pub later: bool,

    /// Per-attempt timeout in seconds (defaults to the `#[job]` macro's `timeout`).
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Max retries when using `--later` (defaults to the `#[job]` macro's `max_retries`).
    #[arg(long)]
    pub max_retries: Option<u32>,
}

/// Exit code for the CLI (`0` success, `1` failure).
pub async fn run(args: RunArgs, queue: Option<Arc<dyn JobQueue>>) -> i32 {
    match run_inner(args, queue).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    }
}

async fn run_inner(args: RunArgs, queue: Option<Arc<dyn JobQueue>>) -> Result<()> {
    let registry = JobRegistry::from_inventory();

    if registry.defaults_for(&args.job_name).is_none() {
        let known = registry.names();
        let mut msg = format!("unknown job `{}`", args.job_name);
        let hints = suggest_job_names(&known, &args.job_name, 5);
        if hints.is_empty() {
            msg.push_str("\nregistered jobs: (none — link a crate that defines `#[job]` handlers)");
        } else {
            msg.push_str("\ndid you mean:");
            for h in &hints {
                msg.push_str("\n  ");
                msg.push_str(h);
            }
        }
        msg.push_str("\nlist handlers with: cargo doido jobs list");
        return Err(anyhow!(msg));
    }

    let payloads = parse_job_payloads_json(args.payload.as_deref())?;

    let jobs = build_job_payloads(
        &registry,
        &args.job_name,
        payloads,
        args.queue.clone(),
        args.timeout,
        args.max_retries,
    )?;
    let queue_name = jobs
        .first()
        .map(|j| j.queue.clone())
        .unwrap_or_else(|| "default".to_string());

    if args.later {
        let queue = queue.ok_or_else(|| {
            anyhow!(
                "jobs backend is not available (check config and DATABASE_URL for the db backend)"
            )
        })?;
        let ids = enqueue_jobs(queue.as_ref(), jobs).await?;
        if ids.len() == 1 {
            println!(
                "enqueued job_id={} queue={queue_name} job_name={}",
                ids[0], args.job_name
            );
        } else {
            println!(
                "enqueued {} job(s) queue={queue_name} job_name={} ids={}",
                ids.len(),
                args.job_name,
                ids.join(", ")
            );
        }
        return Ok(());
    }

    let ctx = Arc::new(job_context_from_runtime());
    perform_jobs_now(&registry, jobs, ctx).await
}
