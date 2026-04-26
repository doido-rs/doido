# Macros Merge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move each `doido-X-macros/` crate inside its parent `doido-X/macros/` so the workspace root only exposes pure lib crates; the parent re-exports all proc macros.

**Architecture:** Each proc-macro crate becomes a subcrate at `doido-X/macros/`. The workspace `members` path updates from `"doido-X-macros"` to `"doido-X/macros"`. The parent's `Cargo.toml` path dep changes from `"../doido-X-macros"` to `"macros"`. The parent's `src/lib.rs` gains `pub use doido_x_macros::*;` so users never import the macros crate directly. Crate *names* are unchanged — no `Cargo.lock` churn.

**Tech Stack:** Rust, Cargo workspaces. No new dependencies.

---

## File Map

| Action | Path |
|--------|------|
| Move dir | `doido-controller-macros/` → `doido-controller/macros/` |
| Move dir | `doido-router-macros/` → `doido-router/macros/` |
| Move dir | `doido-jobs-macros/` → `doido-jobs/macros/` |
| Move dir | `doido-mailer-macros/` → `doido-mailer/macros/` |
| Move dir | `doido-cable-macros/` → `doido-cable/macros/` |
| Move dir | `doido-kafka-macros/` → `doido-kafka/macros/` |
| Move dir | `doido-mcp-macros/` → `doido-mcp/macros/` |
| Modify | `Cargo.toml` (workspace root) |
| Modify | `doido-controller/Cargo.toml`, `doido-controller/src/lib.rs` |
| Modify | `doido-router/Cargo.toml`, `doido-router/src/lib.rs` |
| Modify | `doido-jobs/Cargo.toml`, `doido-jobs/src/lib.rs` |
| Modify | `doido-mailer/Cargo.toml`, `doido-mailer/src/lib.rs` |
| Modify | `doido-cable/Cargo.toml`, `doido-cable/src/lib.rs` |
| Modify | `doido-kafka/Cargo.toml`, `doido-kafka/src/lib.rs` |
| Modify | `doido-mcp/Cargo.toml`, `doido-mcp/src/lib.rs` |

---

## Task 1: Merge `doido-controller-macros` → `doido-controller/macros`

**Files:**
- Move: `doido-controller-macros/` → `doido-controller/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-controller/Cargo.toml`
- Modify: `doido-controller/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-controller-macros doido-controller/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, find the members list and replace:
```toml
# before
    "doido-controller-macros",
# after
    "doido-controller/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-controller/Cargo.toml`**

Change:
```toml
doido-controller-macros = { path = "../doido-controller-macros" }
```
To:
```toml
doido-controller-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-controller/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_controller_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-controller
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-controller/
git commit -m "refactor: move doido-controller-macros into doido-controller/macros"
```

---

## Task 2: Merge `doido-router-macros` → `doido-router/macros`

**Files:**
- Move: `doido-router-macros/` → `doido-router/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-router/Cargo.toml`
- Modify: `doido-router/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-router-macros doido-router/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, replace:
```toml
# before
    "doido-router-macros",
# after
    "doido-router/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-router/Cargo.toml`**

Change:
```toml
doido-router-macros = { path = "../doido-router-macros" }
```
To:
```toml
doido-router-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-router/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_router_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-router
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-router/
git commit -m "refactor: move doido-router-macros into doido-router/macros"
```

---

## Task 3: Merge `doido-jobs-macros` → `doido-jobs/macros`

**Files:**
- Move: `doido-jobs-macros/` → `doido-jobs/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-jobs/Cargo.toml`
- Modify: `doido-jobs/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-jobs-macros doido-jobs/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, replace:
```toml
# before
    "doido-jobs-macros",
# after
    "doido-jobs/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-jobs/Cargo.toml`**

Change:
```toml
doido-jobs-macros = { path = "../doido-jobs-macros" }
```
To:
```toml
doido-jobs-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-jobs/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_jobs_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-jobs
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-jobs/
git commit -m "refactor: move doido-jobs-macros into doido-jobs/macros"
```

---

## Task 4: Merge `doido-mailer-macros` → `doido-mailer/macros`

**Files:**
- Move: `doido-mailer-macros/` → `doido-mailer/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-mailer/Cargo.toml`
- Modify: `doido-mailer/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-mailer-macros doido-mailer/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, replace:
```toml
# before
    "doido-mailer-macros",
# after
    "doido-mailer/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-mailer/Cargo.toml`**

Change:
```toml
doido-mailer-macros = { path = "../doido-mailer-macros" }
```
To:
```toml
doido-mailer-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-mailer/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_mailer_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-mailer
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-mailer/
git commit -m "refactor: move doido-mailer-macros into doido-mailer/macros"
```

---

## Task 5: Merge `doido-cable-macros` → `doido-cable/macros`

**Files:**
- Move: `doido-cable-macros/` → `doido-cable/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-cable/Cargo.toml`
- Modify: `doido-cable/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-cable-macros doido-cable/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, replace:
```toml
# before
    "doido-cable-macros",
# after
    "doido-cable/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-cable/Cargo.toml`**

Change:
```toml
doido-cable-macros = { path = "../doido-cable-macros" }
```
To:
```toml
doido-cable-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-cable/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_cable_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-cable
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-cable/
git commit -m "refactor: move doido-cable-macros into doido-cable/macros"
```

---

## Task 6: Merge `doido-kafka-macros` → `doido-kafka/macros`

**Files:**
- Move: `doido-kafka-macros/` → `doido-kafka/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-kafka/Cargo.toml`
- Modify: `doido-kafka/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-kafka-macros doido-kafka/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, replace:
```toml
# before
    "doido-kafka-macros",
# after
    "doido-kafka/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-kafka/Cargo.toml`**

Change:
```toml
doido-kafka-macros = { path = "../doido-kafka-macros" }
```
To:
```toml
doido-kafka-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-kafka/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_kafka_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-kafka
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-kafka/
git commit -m "refactor: move doido-kafka-macros into doido-kafka/macros"
```

---

## Task 7: Merge `doido-mcp-macros` → `doido-mcp/macros`

**Files:**
- Move: `doido-mcp-macros/` → `doido-mcp/macros/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `doido-mcp/Cargo.toml`
- Modify: `doido-mcp/src/lib.rs`

- [ ] **Step 1: Move the directory**

```bash
git mv doido-mcp-macros doido-mcp/macros
```

- [ ] **Step 2: Update workspace `Cargo.toml`**

In `/Cargo.toml`, replace:
```toml
# before
    "doido-mcp-macros",
# after
    "doido-mcp/macros",
```

- [ ] **Step 3: Update parent path dep in `doido-mcp/Cargo.toml`**

Change:
```toml
doido-mcp-macros = { path = "../doido-mcp-macros" }
```
To:
```toml
doido-mcp-macros = { path = "macros" }
```

- [ ] **Step 4: Add re-export to `doido-mcp/src/lib.rs`**

Append to the end of the file:
```rust
pub use doido_mcp_macros::*;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p doido-mcp
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml doido-mcp/
git commit -m "refactor: move doido-mcp-macros into doido-mcp/macros"
```

---

## Task 8: Final workspace verification

- [ ] **Step 1: Build entire workspace**

```bash
cargo build --workspace
```
Expected: all crates compile, no errors.

- [ ] **Step 2: Run all tests**

```bash
cargo test --workspace
```
Expected: all tests pass.

- [ ] **Step 3: Verify no top-level `*-macros` directories remain**

```bash
ls -d doido-*-macros 2>/dev/null && echo "FAIL: stray macros dirs" || echo "OK: no stray macros dirs"
```
Expected: `OK: no stray macros dirs`

- [ ] **Step 4: Commit if any loose files remain**

If the above checks pass, the work is done. No additional commit needed — each task committed its own changes.
