# Doido autonomous finishing loop — one iteration

You are **one iteration** of an autonomous loop finishing the Doido framework. You
have **no memory** of previous iterations; the only state is `harness/prd.json`, the
git history, `harness/progress.txt`, and the checkboxes in
`docs/RAILS8-GAP-ANALYSIS.md`. Do exactly one story, then stop.

## Steps

1. Read `harness/prd.json`. Pick the story with the **lowest `priority`** whose
   `"passes": false`. If every story passes, print `<promise>COMPLETE</promise>` and stop.
2. Read the spec referenced by the story in `docs/` and skim `docs/ARCHITECTURE.md`.
3. **TDD**: write the failing test(s) that encode the story's `acceptanceCriteria`
   **first**. Run them and confirm they fail for the right reason.
4. Implement the **minimal** code to make them pass. Match the surrounding crate's
   patterns, naming, and comment density.
5. Run `make verify`. It **must** exit 0. If it is red, fix it before doing anything
   else — never end an iteration with a red tree. If the story touched a **generator
   or an embedded template** (`doido-generators/`), also run `make example` and keep
   it green.
6. Tick the story's feature line in `docs/RAILS8-GAP-ANALYSIS.md` from `- [ ]` to
   `- [x]` (the story names the exact line).
7. Update `harness/prd.json`: set this story's `"passes": true` and write a one-line
   `notes`. Append one line to `harness/progress.txt` (`US-XXX: <what changed>`).
8. Commit on branch `first_stable_project`:
   `feat(<crate>): <story title> [US-XXX]`, ending with:
   `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

## Hard rules

- **One story per iteration.** Do not start a second, even if there is time. (Tightly
  coupled features may land in the same diff — e.g. sessions↔flash↔cookies — but each
  keeps its own failing-first test, its own checkbox, and its own `passes` flip.)
- **`make verify` green is the definition of done** for every story. `make verify`
  runs fmt + clippy `-D warnings` + `cargo test --workspace` (~12s); it is
  deterministic and does not depend on the network. The slow generate-and-build e2e
  runs under `make example` (out of `verify` by design).
- **Coverage (`make coverage-check`) is a parallel quality gate** — 80% line coverage
  per workspace crate (`harness/coverage-plan.md`). It stays out of `verify` until all
  crates pass; coverage work adds **tests only** (no `src/` production changes).
- **Supply-chain (`make supply-chain`) is NOT part of the gate** — the RustSec DB is
  time-varying and must not turn the loop red. Run it in CI, not here.
- **Do not weaken tests to pass.** Do not edit a story's `acceptanceCriteria`; only
  flip `passes` and add `notes`.
- **New dependencies must pass `deny.toml`** (license allowlist: MIT/Apache-2.0/BSD/
  ISC/Zlib/Unicode/CDLA). Prefer already-vendored crates; check before adding.
- If a story is **blocked** (needs a human decision, e.g. the config YAML-vs-TOML
  drift in ARCHITECTURE.md), write the blocker to `notes`, leave `passes: false`,
  and stop after stating what you need. Do not guess on product decisions.
