# Doido finishing harness

This directory is the autonomous harness that finishes the Doido framework on top of
the green baseline (`make verify`).

## Files

- `prd.json` — the backlog: one dependency-ordered story per gap-file checkbox, seeded
  from [`docs/RAILS8-GAP-ANALYSIS.md`](../docs/RAILS8-GAP-ANALYSIS.md) (the Rails 8 gap
  analysis). Each story's definition of done is "its tests pass AND `make verify` exits
  0". `passes` tracks completion; completing a story also ticks its `[ ]`→`[x]` box in
  the gap file.
- `LOOP.md` — the contract for a single autonomous iteration (pick next story → TDD →
  implement → `make verify` → tick box → commit → flip `passes`). Tool-agnostic.
- `progress.txt` — append-only log, one line per completed story.
- `archive/` — the previous finishing run (a different US-001..009 backlog + its
  progress log), kept for history.

## The gate

Everything hinges on one deterministic command:

```sh
make verify        # fmt + clippy + tests (~12s); must exit 0
make example       # slow generate-and-build e2e (out of verify by design)
```

Supply-chain audit is intentionally **out** of the loop gate (it depends on the
time-varying RustSec advisory DB). Run it separately / in CI:

```sh
make supply-chain  # cargo-deny + cargo-audit
```

Backend (redis/postgres/memcache) tests are gated and self-skip without services:

```sh
make services-up && make test-backends   # needs docker socket permissions
```

## Running the loop

**Option A — Claude Code `/loop` (self-paced):**

```
/loop  Run one iteration of harness/LOOP.md, then stop.
```

`/loop` re-invokes the prompt each cycle; each iteration reads `prd.json` fresh and
does exactly one story. It ends when a story prints `<promise>COMPLETE</promise>`.

**Option B — Ralph (`ralph.sh`):** point Ralph's `prompt.md`/`prd.json` at these files
(or copy them into the ralph skill dir) and run:

```sh
./ralph.sh --tool claude 20    # up to 20 iterations
```

Ralph stops when an iteration emits `<promise>COMPLETE</promise>`.

## Guardrails

- One story per iteration; branch `first_stable_project`; one commit per story.
- Never end an iteration with `make verify` red.
- Blocked stories (e.g. the config YAML-vs-TOML decision) stop and ask — they are not
  guessed. See `LOOP.md`.
