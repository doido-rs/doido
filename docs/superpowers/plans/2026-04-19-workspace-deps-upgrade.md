# Workspace Dependencies Centralization + sea-orm 2.0 Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Centralize all shared Cargo deps into `[workspace.dependencies]`, upgrade sea-orm to "2", normalize tower to "0.5" and axum to "0.8" across all crates.

**Architecture:** Add a `[workspace.dependencies]` table to the root `Cargo.toml`, then replace every inline dep in member crates with `.workspace = true`. Extra features beyond the workspace baseline are added inline via `features = [...]`. Verification gates: `cargo check` → `cargo test` → `cargo deny check` → `cargo audit`.

**Tech Stack:** Cargo workspace, sea-orm 2, axum 0.8, tower 0.5

---

### Task 1: Add `[workspace.dependencies]` to root `Cargo.toml`

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the workspace dependencies table**

Open `Cargo.toml`. After the `[workspace.package]` section append:

```toml
[workspace.dependencies]
# ORM
sea-orm     = { version = "2", features = ["sqlx-sqlite", "runtime-tokio-rustls", "macros"] }

# Async runtime
tokio       = { version = "1", features = ["rt-multi-thread", "macros"] }
async-trait = "0.1"

# Web
axum        = "0.8"
tower       = { version = "0.5", features = ["util"] }
tower-http  = { version = "0.6", features = ["cors", "trace", "catch-panic"] }
http        = "1"
http-body-util = "0.1"

# Serialization
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"

# Proc-macro helpers
syn         = { version = "2", features = ["full"] }
quote       = "1"
proc-macro2 = "1"

# Observability
tracing     = "0.1"

# Utilities
uuid        = { version = "1", features = ["v4"] }
clap        = { version = "4", features = ["derive"] }
chrono      = { version = "0.4", features = ["clock"] }
```

- [ ] **Step 2: Verify root manifest parses**

```bash
cargo metadata --no-deps --format-version 1 > /dev/null && echo "OK"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add [workspace.dependencies] table"
```

---

### Task 2: Update proc-macro crates

**Files:**
- Modify: `doido-cable-macros/Cargo.toml`
- Modify: `doido-controller-macros/Cargo.toml`
- Modify: `doido-jobs-macros/Cargo.toml`
- Modify: `doido-kafka-macros/Cargo.toml`
- Modify: `doido-mailer-macros/Cargo.toml`
- Modify: `doido-mcp-macros/Cargo.toml`
- Modify: `doido-router-macros/Cargo.toml`

All 7 macro crates have the same deps. Apply identical change to each:

- [ ] **Step 1: Replace inline deps with workspace refs in all macro crates**

For each of the 7 files, replace:
```toml
[dependencies]
proc-macro2 = "1"
quote = "1"
syn = { version = "2", features = ["full"] }
```

With:
```toml
[dependencies]
proc-macro2.workspace = true
quote.workspace = true
syn.workspace = true
```

- [ ] **Step 2: Verify**

```bash
cargo check -p doido-cable-macros -p doido-controller-macros -p doido-jobs-macros \
  -p doido-kafka-macros -p doido-mailer-macros -p doido-mcp-macros -p doido-router-macros
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add doido-cable-macros/Cargo.toml doido-controller-macros/Cargo.toml \
  doido-jobs-macros/Cargo.toml doido-kafka-macros/Cargo.toml \
  doido-mailer-macros/Cargo.toml doido-mcp-macros/Cargo.toml \
  doido-router-macros/Cargo.toml
git commit -m "chore: use workspace deps in proc-macro crates"
```

---

### Task 3: Update `doido-core`

**Files:**
- Modify: `doido-core/Cargo.toml`

- [ ] **Step 1: Replace inline deps**

Replace the `[dependencies]` section with:

```toml
[dependencies]
anyhow = "1"
thiserror = "1"
async-trait.workspace = true
tracing.workspace = true
serde.workspace = true
regex = "1"
```

- [ ] **Step 2: Verify**

```bash
cargo check -p doido-core
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add doido-core/Cargo.toml
git commit -m "chore: use workspace deps in doido-core"
```

---

### Task 4: Update `doido-model` (sea-orm 1 → 2)

**Files:**
- Modify: `doido-model/Cargo.toml`

- [ ] **Step 1: Replace inline deps**

Replace `[dependencies]` and `[dev-dependencies]` with:

```toml
[dependencies]
doido-core = { path = "../doido-core" }
sea-orm.workspace = true
tokio.workspace = true

[dev-dependencies]
tokio.workspace = true
```

- [ ] **Step 2: Check for sea-orm 2 API breakage**

```bash
cargo check -p doido-model
```

Expected: no errors (doido-model is mostly re-exports — if any re-export path changed in sea-orm 2, it will appear here as a compile error; fix the path in `doido-model/src/lib.rs` accordingly)

- [ ] **Step 3: Run model tests**

```bash
cargo test -p doido-model -- --no-capture --test-threads=1
```

Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add doido-model/Cargo.toml
git commit -m "chore: upgrade sea-orm to 2 via workspace dep"
```

---

### Task 5: Update `doido-router` and `doido-controller` (axum 0.7 → 0.8, tower 0.4 → 0.5)

**Files:**
- Modify: `doido-router/Cargo.toml`
- Modify: `doido-controller/Cargo.toml`

- [ ] **Step 1: Update `doido-router/Cargo.toml`**

Replace `[dependencies]` and `[dev-dependencies]` with:

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-router-macros = { path = "../doido-router-macros" }
axum.workspace = true
tokio = { workspace = true, features = ["full"] }

[dev-dependencies]
tower.workspace = true
http-body-util.workspace = true
http.workspace = true
tokio.workspace = true
```

- [ ] **Step 2: Update `doido-controller/Cargo.toml`**

Replace `[dependencies]` and `[dev-dependencies]` with:

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-controller-macros = { path = "../doido-controller-macros" }
axum.workspace = true
tokio = { workspace = true, features = ["full"] }
serde.workspace = true
serde_json.workspace = true
http.workspace = true
serde_urlencoded = "0.7"

[dev-dependencies]
tower.workspace = true
http-body-util.workspace = true
tokio.workspace = true
```

- [ ] **Step 3: Check for axum 0.8 API breakage**

```bash
cargo check -p doido-router -p doido-controller
```

Expected: no errors. If axum 0.8 renamed anything used in `src/`, fix those call sites now.

- [ ] **Step 4: Run tests**

```bash
cargo test -p doido-router -p doido-controller -- --no-capture --test-threads=1
```

Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add doido-router/Cargo.toml doido-controller/Cargo.toml
git commit -m "chore: upgrade axum to 0.8 and tower to 0.5 in router+controller"
```

---

### Task 6: Update `doido-middleware` and `doido-mcp`

**Files:**
- Modify: `doido-middleware/Cargo.toml`
- Modify: `doido-mcp/Cargo.toml`

- [ ] **Step 1: Update `doido-middleware/Cargo.toml`**

Replace `[dependencies]` and `[dev-dependencies]` with:

```toml
[dependencies]
doido-core = { path = "../doido-core" }
async-trait.workspace = true
axum.workspace = true
tower = { workspace = true, features = ["full"] }
tower-http.workspace = true
tracing.workspace = true
serde_json.workspace = true
http.workspace = true

[dev-dependencies]
tokio.workspace = true
http-body-util.workspace = true
tower.workspace = true
```

- [ ] **Step 2: Update `doido-mcp/Cargo.toml`**

Replace `[dependencies]` and `[dev-dependencies]` with:

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-mcp-macros = { path = "../doido-mcp-macros" }
serde.workspace = true
serde_json.workspace = true
axum.workspace = true
tokio.workspace = true
http.workspace = true

[dev-dependencies]
tower.workspace = true
http-body-util.workspace = true
tokio.workspace = true
```

- [ ] **Step 3: Verify**

```bash
cargo check -p doido-middleware -p doido-mcp
```

Expected: no errors

- [ ] **Step 4: Run tests**

```bash
cargo test -p doido-middleware -p doido-mcp -- --no-capture --test-threads=1
```

Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add doido-middleware/Cargo.toml doido-mcp/Cargo.toml
git commit -m "chore: use workspace deps in middleware and mcp"
```

---

### Task 7: Update remaining crates

**Files:**
- Modify: `doido-cache/Cargo.toml`
- Modify: `doido-jobs/Cargo.toml`
- Modify: `doido-cable/Cargo.toml`
- Modify: `doido-kafka/Cargo.toml`
- Modify: `doido-mailer/Cargo.toml`
- Modify: `doido-config/Cargo.toml`
- Modify: `doido-generators/Cargo.toml`
- Modify: `doido-cli/Cargo.toml`
- Modify: `doido-view/Cargo.toml`

- [ ] **Step 1: Update `doido-cache/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
serde_json.workspace = true
async-trait.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["time"] }
doido-core = { path = "../doido-core" }
async-trait.workspace = true
```

- [ ] **Step 2: Update `doido-jobs/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-jobs-macros = { path = "../doido-jobs-macros" }
serde.workspace = true
serde_json.workspace = true
tokio = { workspace = true, features = ["sync", "time"] }
async-trait.workspace = true
uuid.workspace = true

[dev-dependencies]
tokio.workspace = true
```

- [ ] **Step 3: Update `doido-cable/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-cable-macros = { path = "../doido-cable-macros" }
serde.workspace = true
serde_json.workspace = true
async-trait.workspace = true
tokio = { workspace = true, features = ["sync"] }

[dev-dependencies]
tokio.workspace = true
async-trait.workspace = true
serde_json.workspace = true
doido-core = { path = "../doido-core" }
```

- [ ] **Step 4: Update `doido-kafka/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-kafka-macros = { path = "../doido-kafka-macros" }
serde.workspace = true
serde_json.workspace = true
async-trait.workspace = true

[dev-dependencies]
tokio.workspace = true
async-trait.workspace = true
```

- [ ] **Step 5: Update `doido-mailer/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-mailer-macros = { path = "../doido-mailer-macros" }
serde.workspace = true
serde_json.workspace = true
async-trait.workspace = true
tracing.workspace = true
tokio = { workspace = true, features = ["sync"] }

[dev-dependencies]
tokio.workspace = true
```

- [ ] **Step 6: Update `doido-config/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
serde.workspace = true
toml = "0.8"
aes-gcm = "0.10"
base64 = "0.22"
hex = "0.4"

[dev-dependencies]
tempfile = "3"
serial_test = "3"
```

- [ ] **Step 7: Update `doido-generators/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
chrono.workspace = true

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 8: Update `doido-cli/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
doido-generators = { path = "../doido-generators" }
clap.workspace = true

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 9: Update `doido-view/Cargo.toml`**

```toml
[dependencies]
doido-core = { path = "../doido-core" }
tera = "1"
serde_json.workspace = true

[dev-dependencies]
doido-core = { path = "../doido-core" }
tempfile = "3"
```

- [ ] **Step 10: Verify all updated crates**

```bash
cargo check -p doido-cache -p doido-jobs -p doido-cable -p doido-kafka \
  -p doido-mailer -p doido-config -p doido-generators -p doido-cli -p doido-view
```

Expected: no errors

- [ ] **Step 11: Commit**

```bash
git add doido-cache/Cargo.toml doido-jobs/Cargo.toml doido-cable/Cargo.toml \
  doido-kafka/Cargo.toml doido-mailer/Cargo.toml doido-config/Cargo.toml \
  doido-generators/Cargo.toml doido-cli/Cargo.toml doido-view/Cargo.toml
git commit -m "chore: use workspace deps in remaining crates"
```

---

### Task 8: Full verification

**Files:** none (read-only verification)

- [ ] **Step 1: Full workspace check**

```bash
cargo check --workspace
```

Expected: no errors

- [ ] **Step 2: Full test suite**

```bash
cargo test -- --no-capture --test-threads=1
```

Expected: all tests pass

- [ ] **Step 3: License and advisory checks**

```bash
cargo deny check
```

Expected: no errors (warnings about duplicate versions are acceptable)

- [ ] **Step 4: Security audit**

```bash
cargo audit
```

Expected: no errors (RUSTSEC-2023-0071 is ignored via `.cargo/audit.toml`)

- [ ] **Step 5: Commit if any fixups were needed**

```bash
git add -p
git commit -m "fix: address workspace dep migration issues"
```

Only commit if step 1 or 2 required source-level fixes. Skip otherwise.
