+++
title = "One-line Jobs WebUI setup in Doido"
description = "Encapsulating configuration, synchronous queue resolution, mount paths, and auth middleware for background job dashboards."
date = 2026-11-18

[taxonomies]
tags = ["release", "jobs", "webui", "docs"]
+++

Doido background queues (`doido-jobs`) provide a built-in HTTP dashboard for inspecting queue counts, active executions, retry schedules, and dead letters.

Previously, wiring this dashboard into your application router required manual boilerplate: loading YAML configuration, resolving the active queue driver (checking if database connection pools are available or falling back to in-memory queues), normalizing mount URLs, building `JobsWebState`, and attaching authentication middleware.

With the release of `build_jobs_webui_router`, this setup is now encapsulated into a single clean call.

## The problem: 30+ lines of repetitive wiring

Previously, an application needed code similar to this just to construct and protect the dashboard router:

```rust
pub fn build_jobs_webui_router() -> axum::Router {
    let jobs_config = doido_jobs::config::load();
    let queue: std::sync::Arc<dyn doido_jobs::JobQueue> = match jobs_config.backend {
        doido_jobs::config::Backend::Memory => std::sync::Arc::new(
            doido_jobs::MemoryQueue::new().with_enqueue_batch_size(jobs_config.enqueue_batch_size),
        ),
        doido_jobs::config::Backend::Db => {
            if let Some(conn) = doido_model::pool::try_pool().cloned() {
                doido_jobs::config::build_db_queue(conn)
            } else {
                std::sync::Arc::new(
                    doido_jobs::MemoryQueue::new()
                        .with_enqueue_batch_size(jobs_config.enqueue_batch_size),
                )
            }
        }
        #[allow(unreachable_patterns)]
        _ => std::sync::Arc::new(
            doido_jobs::MemoryQueue::new().with_enqueue_batch_size(jobs_config.enqueue_batch_size),
        ),
    };

    let mount_path =
        doido_jobs::webui::normalize_mount(crate::settings::get().jobs.webui.mount_path());
    let state =
        doido_jobs::webui::JobsWebState::new(queue, jobs_config).with_mount_path(&mount_path);

    let mut router = doido_jobs::webui::routes_at(&mount_path, state);
    if !crate::settings::dev_or_test_runtime() {
        router = router.layer(from_fn(crate::auth::jobs_webui_auth_layer));
    }
    router
}
```

## The solution: `build_jobs_webui_router`

`doido-jobs` now provides `build_jobs_webui_router` (and `build_unprotected_jobs_webui_router`), handling the entire lifecycle. The mount path is specified explicitly when calling the builder, without requiring redundant `enabled` or `mount_path` keys in your configuration YAML:

```rust
use axum::middleware::from_fn;

pub fn build_jobs_webui_router() -> axum::Router {
    let auth_layer = (!crate::settings::dev_or_test_runtime())
        .then(|| from_fn(crate::auth::jobs_webui_auth_layer));

    doido_jobs::webui::build_jobs_webui_router("/admin/jobs", auth_layer)
}
```

And for unprotected environments or local prototypes, `build_unprotected_jobs_webui_router("/admin/jobs")` requires zero generic type annotations:

```rust
let router = doido_jobs::webui::build_unprotected_jobs_webui_router("/admin/jobs");
```

## Synchronous queue resolution: `default_sync_queue`

Under the hood, queue resolution is also exported as `doido_jobs::config::default_sync_queue(&jobs_config)`. It inspects `jobs.type` from your configuration:
- For `memory`, it instantiates `MemoryQueue` with the configured batch size.
- For `db` (with the `jobs-db` feature enabled), it checks if a connection is ready via `doido_model::pool::try_pool()` and wraps it in `DbQueue`; if no pool is installed yet, it safely falls back to `MemoryQueue`.
- For other drivers, it provides a safe memory fallback.

## Learn more

Read the full background jobs guide in the [Jobs reference](/docs/reference/jobs/) manual.
