# CI/CD: GitHub Actions + crates.io Publish + Binary Release

**Date:** 2026-04-18
**Status:** Approved

## Goal

Add GitHub Actions workflows that lint, test, publish all 22 crates to crates.io, and release
cross-compiled binaries for Linux, macOS, and Windows — triggered manually via `workflow_dispatch`.

## New `doido` Crate

Add `doido/` as a new workspace member. It serves two purposes:

- **Binary** (`src/main.rs`): CLI entry point; delegates entirely to `doido-cli`.
- **Library** (`src/lib.rs`): umbrella re-exports all public APIs from every other crate
  (`pub use doido_core::*`, `pub use doido_router::*`, etc.).

```
doido/
├── Cargo.toml   # [lib] + [[bin]], depends on all 21 sibling crates
├── src/
│   ├── lib.rs   # pub use doido_*::*; for each crate
│   └── main.rs  # doido_cli::run()
```

`doido/` is added to `[workspace] members` in the root `Cargo.toml`.

## Workflows

Workflow files live in `.github/workflows/` (`ci.yml`, `release.yml`, and `release-delete.yml` for cleanup).

### 1. `ci.yml` — Continuous Integration

Triggers on every push and pull request to any branch.

Jobs (sequential, fail-fast):

1. **lint**
   - `cargo fmt --check --all`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo deny check`
   - `cargo audit`

2. **test**
   - `cargo test --workspace`

### 2. `release.yml` — Publish & Release

Trigger: `workflow_dispatch` with inputs:

| Input | Type | Description |
|-------|------|-------------|
| `version` | string | Semver string applied to all crates (e.g. `0.1.0`; when `unstable=true`, use a suffix such as `0.0.25-test.1`) |
| `unstable` | boolean | Test release — GitHub pre-release (not `latest`); clean up with `release-delete.yml` + `make yank` |
| `dry_run` | boolean | If true, skips actual publish and GitHub Release creation |

Jobs (sequential, fail-fast):

1. **lint** — same as `ci.yml` lint job
2. **test** — `cargo test --workspace`
3. **publish**
   - Install `cargo-workspaces`
   - Bump all crate versions: `cargo workspaces version --exact --yes <version>`
   - Publish in dependency order: `cargo workspaces publish --verbose --yes`
   - Skipped when `dry_run: true`
4. **build** — matrix across 5 targets (see below), produces `doido` binary artifacts
5. **release**
   - Creates a GitHub Release tagged `v<version>`
   - When `unstable: true`, marks the release as a GitHub pre-release (not promoted to `latest`)
   - Attaches all 5 binary artifacts
   - Skipped when `dry_run: true`

### 3. `release-delete.yml` — Remove unstable release

Trigger: `workflow_dispatch` with inputs:

| Input | Type | Description |
|-------|------|-------------|
| `version` | string | Semver to remove (no leading `v`, e.g. `0.0.25-test.1`) |
| `yank_crates` | boolean | If true (default), runs `make yank VERSION=<version>` on every workspace crate |

Jobs:

1. **delete-github** — deletes the GitHub Release and remote tag `v<version>` (no-op if missing)
2. **yank** — yanks the version on crates.io (skipped when `yank_crates: false`)

## Unstable releases

Use an unstable release to publish a test version that should not become the default install target.

1. Run **Release** with `unstable: true` and a pre-release semver (e.g. `0.0.25-test.1`).
   `scripts/bump-workspace-version.sh` rejects a bare `X.Y.Z` when `UNSTABLE=1`.
2. The workflow still publishes to crates.io and builds binaries, but the GitHub Release is
   marked **pre-release**, so `scripts/install.sh` (which resolves `/releases/latest`) keeps
   pointing at the last stable release.
3. When done testing, run **Release delete** with the same version (and `yank_crates: true`).
   Or manually delete the GitHub Release + tag and run `make yank VERSION=0.0.25-test.1`.

Note: crates.io has no hard delete — `cargo yank` (via `make yank`) is the supported way to
pull a version from new dependency resolution.

## Binary Build Matrix

| Runner | Target | Notes |
|--------|--------|-------|
| `ubuntu-latest` | `x86_64-unknown-linux-gnu` | native |
| `ubuntu-latest` | `aarch64-unknown-linux-gnu` | via `cross` |
| `macos-latest` | `x86_64-apple-darwin` | native |
| `macos-latest` | `aarch64-apple-darwin` | native (Apple Silicon runner) |
| `windows-latest` | `x86_64-pc-windows-msvc` | native |

Each build job:
1. Checks out repo
2. Installs Rust stable + target
3. Installs `cross` (Linux ARM only)
4. Builds: `cargo build --release --bin doido` (or `cross build` for ARM Linux)
5. Uploads artifact named `doido-<target>[.exe]`

## Secrets Required

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token for publishing |
| `GITHUB_TOKEN` | built-in; used for creating GitHub Releases |

## Success Criteria

- `ci.yml` runs on every push/PR; lint fails fast before tests.
- `release.yml` triggered manually with a version string publishes all 22 crates in correct dependency order with verbose output visible in logs.
- `dry_run: true` completes the full pipeline without publishing or creating a release.
- Five binary artifacts attached to the GitHub Release, one per target.
- No crate is published with `pub` visibility added beyond what already exists.
