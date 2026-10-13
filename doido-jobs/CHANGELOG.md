# Changelog — doido-jobs

All notable changes to `doido-jobs` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every crate in the `doido` workspace shares one version; the release process is
documented in `RELEASING.md` at the repository root.

> Background jobs, queue backends, and retries (Active Job analogue).

## Unreleased

_No user-facing changes yet._

## 0.0.32 - 2026-10-13

### Changed

- Database-backed queue (`DbQueue`) reads and writes through a SeaORM entity and
  `doido-model` migration helpers instead of raw SQL.
- `doido_jobs` uses an auto-increment integer primary key (`id`); the worker-facing
  job identifier is stored in a unique `job_id` column. `JobQueue` methods still
  take the same string job IDs as before.

## 0.0.24 - 2026-08-14

- Baseline changelog. Releases up to and including `0.0.24` are recorded only in
  the git tags (`v0.0.*`) and their GitHub Releases; per-crate history starts here.
