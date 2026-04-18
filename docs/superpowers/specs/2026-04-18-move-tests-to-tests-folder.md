# Move Inline Tests to `tests/` Folder

**Date:** 2026-04-18  
**Status:** Approved

## Goal

Remove all `#[cfg(test)]` blocks from `src/` files across every Doido crate. Each `src/<module>.rs` that currently contains inline tests gets a corresponding `tests/<module>_test.rs` file. No test code remains in `src/`.

## Naming Convention

For each `src/<module>.rs` with a `#[cfg(test)]` block, create `tests/<module>_test.rs`. Existing integration test files (e.g., `tests/cache_test.rs`) are left unchanged; new per-module files are added alongside them.

```
src/memory.rs   (inline tests removed)  →  tests/memory_test.rs  (new)
src/store.rs    (inline tests removed)  →  tests/store_test.rs   (new)
tests/cache_test.rs                     →  tests/cache_test.rs   (unchanged)
```

## Visibility Strategy

Private items needed by tests are exposed with the minimum necessary `pub(crate)` scope — applied to the specific field, function, or type, not the whole module. Items only accessed via public API require no visibility change.

```rust
// Only expose what tests actually need:
pub(crate) fn to_snake_case(s: &str) -> String { ... }

pub struct MemoryStore {
    pub(crate) data: RwLock<HashMap<...>>,  // only if test directly accesses field
}
```

## Crates and Files Affected

Macro crates are skipped (no tests). 28 `src/` files across 11 crates:

| Crate | `src/` modules to migrate |
|-------|--------------------------|
| `doido-cable` | `cable.rs`, `channel.rs`, `protocol.rs`, `pubsub.rs` |
| `doido-cache` | `memory.rs`, `namespaced.rs`, `registry.rs`, `store.rs` |
| `doido-config` | `crypto.rs`, `env_override.rs`, `loader.rs`, `types.rs` |
| `doido-core` | `inflector/inflections.rs` |
| `doido-jobs` | `memory.rs`, `queue.rs` |
| `doido-kafka` | `codec.rs`, `consumer.rs` |
| `doido-mailer` | `deliverer.rs`, `mail.rs` |
| `doido-mcp` | `protocol.rs`, `registry.rs` |
| `doido-middleware` | `session.rs`, `stack.rs` |
| `doido-model` | `testing.rs` |
| `doido-view` | `engine.rs`, `renderer.rs`, `response.rs`, `tera_engine.rs` |

`doido-router`, `doido-generators`, `doido-cli` — no inline tests, skip.

## Migration Steps Per File

For each `src/<module>.rs`:

1. Identify what private items the `#[cfg(test)]` block uses.
2. Add `pub(crate)` to only those items in `src/<module>.rs`.
3. Create `tests/<module>_test.rs` with the test content, updating imports to use the crate's public/pub(crate) API.
4. Remove the `#[cfg(test)]` block from `src/<module>.rs`.
5. Run `cargo test -p <crate>` — must pass before moving to the next file.

## Success Criteria

- `cargo test --workspace` passes with zero failures.
- `grep -rn "#\[cfg(test)\]" --include="*.rs" */src/` returns no matches in any crate.
- No new `pub` items — only `pub(crate)` where strictly required.
