# Changelog — doido-generators

All notable changes to `doido-generators` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every crate in the `doido` workspace shares one version; the release process is
documented in `RELEASING.md` at the repository root.

> Code generators plus the runtime CLI (server, db, worker, credentials).

## Unreleased

_No user-facing changes yet._

## 0.0.32-test2 - 2026-10-15

_No user-facing changes yet._

## 0.0.32-test1 - 2026-10-14

_No user-facing changes yet._

## 0.0.32 - 2026-10-13

### Changed

- Database-jobs bootstrap migration (`m20260101000001_create_doido_jobs_table`)
  creates `doido_jobs` with `create_table` and `add_index`: implicit bigint `id`
  primary key plus unique `job_id`, replacing the previous raw SQL schema with a
  TEXT primary key.

## 0.0.24 - 2026-08-14

- Baseline changelog. Releases up to and including `0.0.24` are recorded only in
  the git tags (`v0.0.*`) and their GitHub Releases; per-crate history starts here.
