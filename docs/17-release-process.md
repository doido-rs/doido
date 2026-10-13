# 17 — Release Process (branching, versioning, changelogs)

The **delivery** model for Doido — a Rails-style, release-driven process layered on
GitHub. It defines the branch architecture keyed to releases, Semantic Versioning
driven by Conventional Commits, per-crate changelogs, and the maintenance window for
older majors. Unlike the other numbered docs this one describes *process*, not a
crate; the operational runbook and the copy-paste commands live in the repository's
[`RELEASING.md`](../RELEASING.md), and the contributor-facing summary in
[`CONTRIBUTING.md`](../CONTRIBUTING.md).

> **Status (2026-08-14): adopted.** `develop` is the integration trunk and default
> branch; `master` is the stable pointer to the latest release of the highest major;
> ephemeral `release/X.Y.Z` branches are the only thing that publishes and are deleted
> in favor of the `vX.Y.Z` tag. Each of the 18 published crates carries its own
> `CHANGELOG.md`. Tooling: `scripts/release-prep.sh` (bump + stamp + commit) and
> `scripts/release-notes.sh` (assemble the GitHub Release body); CI enforces
> release-branch-only publishing.

## Design intent

- **One version, many crates.** The workspace publishes atomically at a single
  `[workspace.package].version`; branching and changelogs are organized around that
  single version, not per-crate versions.
- **Trunk vs. stable pointer.** All work integrates on `develop` (squash-merge). The
  released state of the newest major is mirrored to `master`; older majors live on
  `<major>-stable` branches (post-1.0). This gives a **last-3-majors** support window.
- **Releases are branches that become tags.** Publishing happens only from
  `release/X.Y.Z`; after crates.io + binaries + the GitHub Release, the branch is
  deleted and the immutable `vX.Y.Z` tag is the record.
- **The tree never lies.** The version bump and changelog stamp are *committed on the
  release branch* (`scripts/release-prep.sh`) so the sources match the published tag —
  replacing the earlier un-committed in-CI bump.

## Version semantics

Conventional Commits since the last tag decide the bump: `fix:` → patch, `feat:` →
minor, `feat!:`/`BREAKING CHANGE:` → major. While in **0.x**, the line is treated as
a single "major 0": breaking → minor (`0.Y.0`), otherwise → patch (`0.0.Z`); the
multi-major maintenance policy activates at `1.0.0`.

## See also

- [`RELEASING.md`](../RELEASING.md) — full runbook, branch-protection setup, housekeeping.
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — day-to-day PR flow and changelog requirement.
- [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) — implementation-state architecture.
- `.github/workflows/release.yml` — the publishing pipeline that enforces this process.
