//! `doido jobs run` / `perform` — in-process perform or enqueue for the worker.

use crate::context::job_context_from_runtime;
use crate::registry::{build_job_payload, suggest_job_names, JobRegistry};
use crate::worker::perform_job_now;
use crate::JobQueue;
use clap::Args;
use doido_core::{anyhow::anyhow, Result};
use serde_json::Value;
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

    let payload = parse_payload(args.payload.as_deref())?;

    let job = build_job_payload(
        &registry,
        &args.job_name,
        payload,
        args.queue.clone(),
        args.timeout,
        args.max_retries,
    )?;
    let queue_name = job.queue.clone();

    if args.later {
        let queue = queue.ok_or_else(|| {
            anyhow!(
                "jobs backend is not available (check config and DATABASE_URL for the db backend)"
            )
        })?;
        let id = queue.enqueue(job).await?;
        println!(
            "enqueued job_id={id} queue={queue_name} job_name={}",
            args.job_name
        );
        return Ok(());
    }

    let ctx = Arc::new(job_context_from_runtime());
    perform_job_now(&registry, job, ctx).await
}

fn parse_payload(raw: Option<&str>) -> Result<Value> {
    match raw {
        None | Some("") => Ok(Value::Object(Default::default())),
        Some(s) => serde_json::from_str(s).map_err(|e| anyhow!("invalid JSON for --payload: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_payload_defaults_to_empty_object() {
        assert_eq!(parse_payload(None).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn parse_payload_accepts_scalar_json() {
        assert_eq!(parse_payload(Some("1")).unwrap(), serde_json::json!(1));
    }
}
