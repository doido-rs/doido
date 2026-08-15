# Releasing Doido

Doido follows a Rails-style, release-driven delivery process. This document is the
authoritative runbook for **branching**, **versioning**, **changelogs**, the
**maintenance/support window**, and the **step-by-step release**.

The whole workspace ships as **one version**: `[workspace.package].version` in the
root `Cargo.toml` is mirrored into every first-party `[workspace.dependencies]`
entry, and all 18 crates publish to crates.io together in dependency order
(`scripts/publish-crates.txt`).

---

## Branch architecture

| Branch | Lifetime | Role |
|--------|----------|------|
| `develop` | permanent | **Integration trunk** and GitHub default branch. Every feature/fix PR **squash-merges** here. Targets the next release. |
| `master` | permanent | **Stable pointer**: the latest published release of the **highest** major. Never receives PRs directly — only the release process advances it. During 0.x it tracks the latest `0.0.x` release. |
| `<N-1>-stable`, `<N-2>-stable` | permanent (post-1.0) | Maintenance lines for the two prior majors — the **last-3-majors** support window (`master` covers major `N`). **Not created while in 0.x.** |
| `release/X.Y.Z` | ephemeral | The **only** branch that publishes. Cut from `develop` (top line) or a `*-stable` line (older-major patch). **Deleted after publish** — the `vX.Y.Z` tag is the permanent record. |
| `feature/*`, `fix/*` | ephemeral | Branched off `develop`, squash-merged back. |

```
develop ──●──●──●──────────────●──►   trunk; squash-merges; next release
           \                   (cut)
            release/3.4.0 ──●──►       publish → tag v3.4.0 → merge back → delete
                             \
master  ─────────────────────●──►      = latest release of the highest major (3)
2-stable ──●───────●─────────────►     backport cherry-picks → release/2.x.y
1-stable ──●─────────────────────►     oldest still-supported major
```

### Fix-forward, then backport

Every change lands on `develop` first (squash). To ship a fix in an older
maintained major, **cherry-pick the squashed commit** onto that major's `*-stable`
branch via a backport PR (label `backport`), then let it ride that series' next
patch release. While in 0.x there is nothing to backport to — fixes ride the next
release off `develop`.

### Major rollover (post-1.0)

When the top major bumps `N → N+1`, **snapshot `<N>-stable` from `master`'s last
`vN.*` state before advancing `master` to `v(N+1).0.0`**. This preserves the old
line for backports while keeping master's invariant ("latest of the latest major").

---

## Versioning policy (SemVer)

The bump is chosen at release-cut time from the [Conventional Commits](https://www.conventionalcommits.org/)
since the previous tag:

| Commit type | Bump |
|-------------|------|
| `fix:` | patch |
| `feat:` | minor |
| `feat!:` / `BREAKING CHANGE:` | major |

**0.x rule (current phase).** `0.x` is treated as a single **"major 0"**: breaking
changes bump the **minor** (`0.Y.0`), everything else bumps the **patch** (`0.0.Z`).
There are no `*-stable` lines and no multi-major maintenance until `1.0.0` ships.

The committed version is the source of truth: `scripts/release-prep.sh` writes it
into the tree **on the release branch and commits it**, so the published tag always
matches the sources (this replaces the old, un-committed in-CI bump).

---

## Maintenance / support window

- **During 0.x:** only the current line is supported (develop → master). No backports.
- **From 1.0.0:** bug and security fixes are delivered for the **last 3 majors** —
  `master` (major `N`) plus `<N-1>-stable` and `<N-2>-stable`. When major `N+1`
  releases and pushes `<N-2>` out of the window, that branch becomes EOL (frozen; no
  further releases).

---

## Changelogs

Each published crate keeps its own `CHANGELOG.md`
([Keep a Changelog](https://keepachangelog.com/) + SemVer), newest-first.

- **Authoring:** every PR that changes a crate adds a bullet under that crate's
  `## Unreleased` (`### Added` / `### Changed` / `### Fixed` / `### Removed`). The PR
  template has a checkbox for this. Internal macro/route-dsl sub-crates may record
  `- No user-facing changes.` — that's expected.
- **Stamping:** at release time `scripts/release-prep.sh` renames each crate's
  `## Unreleased` to `## X.Y.Z - <date>` and inserts a fresh empty `Unreleased`.
- **Release notes:** `scripts/release-notes.sh X.Y.Z` assembles every crate's
  `X.Y.Z` section into the GitHub Release body.

---

## Release runbook

Prerequisites: a crates.io token (`CARGO_REGISTRY_TOKEN` secret is already wired
into the workflow) and push access.

1. **Choose the version** `X.Y.Z` from the conventional commits since the last tag
   (see the table above; remember the 0.x rule).
2. **Cut the release branch** from the right base:
   ```bash
   git switch develop && git pull            # top line
   git switch -c release/X.Y.Z
   # …or, post-1.0 older-major patch:
   # git switch <major>-stable && git switch -c release/X.Y.Z
   ```
3. **Prepare** — bump the version, stamp every changelog, and commit:
   ```bash
   make release-prep VERSION=X.Y.Z
   git push -u origin release/X.Y.Z
   ```
   Open a PR `release/X.Y.Z → master` (or → the `*-stable` line) to review the final
   diff (version + stamped changelogs).
4. **Green gate:**
   ```bash
   make verify        # lint + tests + coverage + installer harness
   make release-e2e   # generators + server + HTTP
   ```
5. **Publish** — run the **Release** workflow (Actions → *Release* → *Run workflow*)
   **from the `release/X.Y.Z` branch**, with `version = X.Y.Z`. Use `dry_run: true`
   once first, then `dry_run: false`. The workflow refuses any ref that is not
   `release/*` and refuses a `version` that does not match the committed tree. On
   success it publishes crates.io, builds the binaries, tags `vX.Y.Z`, and creates
   the GitHub Release from the assembled per-crate notes.
6. **Merge back** (PRs, so the version + changelog commit propagates):
   `release/X.Y.Z → master` (if top major) and/or the `*-stable` line, **then →
   `develop`**.
7. **Delete the release branch** once merged — the tag is the permanent artifact:
   ```bash
   git push origin --delete release/X.Y.Z
   ```

---

## One-time GitHub configuration

Do these once when adopting this model (require admin):

```bash
# 1. Create the trunk from the current master and make it the default branch.
git switch master && git pull
git switch -c develop && git push -u origin develop
gh api -X PATCH repos/doido-rs/doido -f default_branch=develop

# 2. Protect the trunk: PRs only, green CI, linear (squash) history.
gh api -X PUT repos/doido-rs/doido/branches/develop/protection --input - <<'JSON'
{ "required_status_checks": { "strict": true, "contexts": ["Lint","Test","Coverage","Release e2e"] },
  "enforce_admins": false,
  "required_pull_request_reviews": { "required_approving_review_count": 1 },
  "required_linear_history": true,
  "restrictions": null }
JSON

# 3. Protect master (and, post-1.0, each *-stable): advanced only via the release PR.
gh api -X PUT repos/doido-rs/doido/branches/master/protection --input - <<'JSON'
{ "required_status_checks": { "strict": true, "contexts": ["Lint","Test","Coverage","Release e2e"] },
  "enforce_admins": false,
  "required_pull_request_reviews": { "required_approving_review_count": 1 },
  "required_linear_history": true,
  "restrictions": null }
JSON
```

Also: set the repository merge button to **squash only** (Settings → General →
Pull Requests), and retarget any open PRs from `master` onto `develop`.

---

## Housekeeping notes

- **`v0.1.0` is not a real release.** The latest legitimate release is `v0.0.24`.
  The stray tag is removed (`git push origin :refs/tags/v0.1.0`); if `0.1.0` was
  ever pushed to crates.io it is yanked (`make yank VERSION=0.1.0`), which **burns**
  that number — the next minor must then be `0.2.0`.
- The committed workspace version is reconciled to the last released version so the
  tree never lies about what shipped.
