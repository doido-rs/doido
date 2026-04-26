# Design: Merge `*-macros` Crates into Parent Crates

**Date:** 2026-04-25  
**Status:** Approved

## Goal

Make all crates except `doido` pure lib crates from the workspace root's perspective. Each `doido-X-macros` proc-macro crate becomes a subcrate living inside its parent's directory, invisible to users.

## Constraint

Rust requires proc-macro crates to be separate compilation units (`[lib] proc-macro = true` cannot coexist with normal library items). The macros crate cannot be literally deleted — it must remain a separate Cargo crate. The solution is to move it inside the parent's directory and have the parent re-export its macros.

## Affected Pairs (7 total)

| Macros crate (current) | New location |
|---|---|
| `doido-controller-macros/` | `doido-controller/macros/` |
| `doido-router-macros/` | `doido-router/macros/` |
| `doido-jobs-macros/` | `doido-jobs/macros/` |
| `doido-mailer-macros/` | `doido-mailer/macros/` |
| `doido-cable-macros/` | `doido-cable/macros/` |
| `doido-kafka-macros/` | `doido-kafka/macros/` |
| `doido-mcp-macros/` | `doido-mcp/macros/` |

Crate names (e.g. `doido-mcp-macros`) are **unchanged** — only the directory path moves.

## Layout After Migration

```
doido-mcp/
  Cargo.toml         ← path dep updated to "macros"
  src/
    lib.rs           ← adds: pub use doido_mcp_macros::*;
  macros/
    Cargo.toml       ← unchanged (proc-macro = true)
    src/
      lib.rs         ← unchanged
```

## Changes Required Per Pair

### 1. Workspace `Cargo.toml`

Replace each `"doido-X-macros"` member entry with `"doido-X/macros"`:

```toml
# before
"doido-mcp-macros",
"doido-mcp",

# after
"doido-mcp/macros",
"doido-mcp",
```

### 2. Parent `Cargo.toml` path dependency

```toml
# before
doido-mcp-macros = { path = "../doido-mcp-macros" }

# after
doido-mcp-macros = { path = "macros" }
```

### 3. Parent `src/lib.rs` re-export

```rust
pub use doido_mcp_macros::*;
```

This makes all proc macros available via the parent crate, so users never import from the macros crate directly.

## Migration Steps (per pair, any order)

1. `mv doido-X-macros/ doido-X/macros/`
2. Update `doido-X/Cargo.toml` — path dep `"../doido-X-macros"` → `"macros"`
3. Update workspace `Cargo.toml` — member `"doido-X-macros"` → `"doido-X/macros"`
4. Add `pub use doido_X_macros::*;` to `doido-X/src/lib.rs`
5. `cargo check`

All 7 pairs are independent and can be done in any order.

## What Does NOT Change

- Crate names (no `Cargo.lock` churn)
- Macro source files (zero logic changes)
- User-facing import paths (users already use the parent crate)
