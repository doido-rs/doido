# Workspace Dependencies Centralization + sea-orm 2.0 Upgrade

**Date:** 2026-04-19
**Branch:** feat_creating_publish_workflow

## Goal

Centralize all shared Cargo dependencies into `[workspace.dependencies]` in the root
`Cargo.toml`, mirroring the existing `version`/`license` consolidation. Simultaneously
upgrade `sea-orm` to `"2"` and normalize `tower` to `"0.5"` across all crates.

## Motivation

- Multiple crates declare the same dep with slightly different versions (`tower "0.4"` vs
  `"0.5"`). A workspace table makes drift impossible.
- Upgrading sea-orm or any other shared dep requires changing one line instead of hunting
  across 20+ `Cargo.toml` files.

## Workspace Dependencies Table

Add to root `Cargo.toml`:

```toml
[workspace.dependencies]
sea-orm    = { version = "2", features = ["sqlx-sqlite", "runtime-tokio-rustls", "macros"] }
tokio      = { version = "1", features = ["rt-multi-thread", "macros"] }
serde      = { version = "1", features = ["derive"] }
tower      = { version = "0.5" }
tower-http = { version = "0.6", features = ["cors", "trace", "catch-panic"] }
syn        = { version = "2", features = ["full"] }
uuid       = { version = "1", features = ["v4"] }
clap       = { version = "4", features = ["derive"] }
chrono     = { version = "0.4", features = ["clock"] }
```

The workspace entry is the minimum common feature set. Member crates that need additional
features extend with `features = [...]` alongside `workspace = true`.

## Member Crate Changes

Each crate replaces inline dep declarations with `dep.workspace = true`.

Example — before:
```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
```

After:
```toml
tokio = { workspace = true, features = ["sync", "time"] }
```

The extra features (`sync`, `time`) stack on top of the workspace base.

## Upgrade Notes

- **sea-orm 2.0**: `doido-model` is mostly re-exports; no deep integration code to migrate.
  If any re-export paths changed in sea-orm 2.0, `cargo check` will surface them immediately.
- **tower 0.5**: `doido-router` and `doido-controller` move from `0.4` to `0.5`.
  Both use only `tower::ServiceBuilder` / `tower::util`, which are stable across versions.

## Verification

1. `cargo check --workspace` — catch any API breakage
2. `cargo test -- --no-capture --test-threads=1` — all tests pass
3. `cargo deny check` — license/advisory checks pass
4. `cargo audit` — no new vulnerabilities
