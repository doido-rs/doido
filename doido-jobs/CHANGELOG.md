# Changelog — doido-jobs

All notable changes to `doido-jobs` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every crate in the `doido` workspace shares one version; the release process is
documented in `RELEASING.md` at the repository root.

> Background jobs, queue backends, and retries (Active Job analogue).

## Unreleased

- Jobs Web UI **Enqueue** tab: select a registered job, optional payload/queue overrides, enqueue now, schedule (`enqueue_at`, UTC), or perform now in-process
- DbQueue (MySQL/PostgreSQL): parallel job reserve via `FOR UPDATE SKIP LOCKED`
- **Performance:** Jobs Web UI — `queue_counts` only on the Queues tab; optional `jobs.webui.stats_ttl_secs`; `include_payload` query param (payload omitted by default); lighter DB `list_jobs` and aggregated SQL `queue_counts`; Redis batched `MGET` and chunked ready-set scans; memory driver lists pages without cloning full buckets
- **Changed:** queue backends moved to `doido-jobs/src/drivers/` (public `doido_jobs::{db,memory,redis}` re-exports unchanged)
- **Changed:** `JobQueue::list_jobs` takes a `light` flag to skip heavy payload fields for dashboard lists

## 0.0.24 - 2026-08-14

- Baseline changelog. Releases up to and including `0.0.24` are recorded only in
  the git tags (`v0.0.*`) and their GitHub Releases; per-crate history starts here.
