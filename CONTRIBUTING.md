# Contributing to Doido

Thanks for helping build Doido! This project uses a Rails-style, release-driven
workflow. The full branching/versioning/release model lives in
[`RELEASING.md`](RELEASING.md) — this file is the day-to-day contributor guide.

## Branching model (short version)

- **`develop`** is the integration trunk and the **default branch**. Open your PR
  against `develop`.
- **`master`** is a protected pointer to the latest published release of the highest
  major — **do not** target PRs at it.
- Post-1.0, `<major>-stable` branches carry maintenance for older majors.

## Making a change

1. Branch off `develop`:
   ```bash
   git switch develop && git pull
   git switch -c feature/short-description   # or fix/short-description
   ```
2. Write the code **TDD-first** (see `CLAUDE.md` and `docs/00-overview.md`).
3. **Add a changelog entry.** For every crate your change affects, add a bullet
   under that crate's `## Unreleased` section in its `CHANGELOG.md`
   (`### Added` / `### Changed` / `### Fixed` / `### Removed`). Keep it short and
   user-facing; end with `(#PR, @you)`. Internal macro/route-dsl sub-crates with no
   user-visible change may use `- No user-facing changes.`
4. Make sure the green gate passes:
   ```bash
   make verify        # lint + tests + coverage + installer harness
   ```
5. Open a PR **into `develop`**. It will be **squash-merged**, so write a
   [Conventional Commit](https://www.conventionalcommits.org/) PR title — it becomes
   the squashed commit message and drives the next version bump:

   | Title prefix | Effect on the next release |
   |--------------|----------------------------|
   | `fix: …` | patch (`0.0.Z` while in 0.x) |
   | `feat: …` | minor (`0.Y.0` while in 0.x) |
   | `feat!: …` or a `BREAKING CHANGE:` footer | major (post-1.0) |
   | `docs:`, `chore:`, `refactor:`, `style:`, `test:` | no release on their own |

## Commit conventions

- Conventional Commits with an optional scope, e.g. `feat(auth): add JWT refresh`.
- Co-author trailer for AI-assisted commits (see the repository convention).

## Backports (post-1.0)

Fixes land on `develop` first. To ship a fix in a still-supported older major, label
the PR `backport` and, after it merges, cherry-pick the squashed commit onto the
relevant `<major>-stable` branch via a backport PR. See
[`RELEASING.md`](RELEASING.md#fix-forward-then-backport).

## Releases

Contributors don't cut releases directly. Releases go out **only** from
`release/X.Y.Z` branches through the Release workflow; the process is documented in
[`RELEASING.md`](RELEASING.md#release-runbook).
