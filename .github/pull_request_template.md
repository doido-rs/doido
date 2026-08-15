<!--
Target branch: develop (the integration trunk). See CONTRIBUTING.md.
PR title must be a Conventional Commit — it becomes the squashed commit message
and drives the next version bump (fix: → patch, feat: → minor, feat!: → major).
-->

## What & why

<!-- Briefly describe the change and the motivation. Link issues with #123. -->

## Checklist

- [ ] Targets `develop` (not `master`/`*-stable`).
- [ ] PR title is a Conventional Commit (`fix:`, `feat:`, `feat!:`, `docs:`, …).
- [ ] Added a `## Unreleased` entry in the `CHANGELOG.md` of **each affected crate**
      (or `- No user-facing changes.` for internal-only crates).
- [ ] `make verify` passes locally.
- [ ] Tests added/updated (TDD-first).

<!-- Backport? Label this PR `backport` and see RELEASING.md#fix-forward-then-backport. -->
