# Doido workspace Makefile
#
# `make publish` uploads the whole workspace to crates.io one crate at a time in
# dependency order. It is resumable and rate-limit aware:
#   * crates already on the registry at the current version are skipped, so a
#     re-run continues where a previous one stopped — no version bump needed;
#   * a crates.io "429 Too Many Requests" (the new-crate rate limit you hit when
#     first-publishing many new crate names) is handled by sleeping until the
#     server's "try again after" time and retrying, instead of failing.
# So a first-time multi-crate publish that trips the rate limit just pauses and
# continues. `make publish-dry-run` validates the workspace without uploading.

# Extra flags forwarded to every `cargo publish` invocation
# (e.g. `make publish PUBLISH_FLAGS=--no-verify`).
PUBLISH_FLAGS ?=

# Seconds to sleep between crate uploads. 62s keeps just over crates.io's
# one-per-minute publish rate and gives the index time to propagate each crate
# before its dependents are published. (Crates skipped because they're already
# published don't incur this sleep.)
PUBLISH_INTERVAL ?= 62

# Extra seconds added on top of the server's "try again after" time when waiting
# out a 429, to absorb clock skew and index propagation.
PUBLISH_RETRY_BUFFER ?= 15

# Isolated, always-wiped target dir for packaging + verification. Keeping it
# separate from the normal `target/` guarantees cargo's verify step compiles the
# freshly packaged crates instead of reusing stale build artifacts from an
# earlier run with different sources (e.g. after a crate is merged or renamed).
PUBLISH_TARGET_DIR ?= target/publish

# Version to (un)yank. Defaults to the current workspace version when empty.
VERSION ?=

# Extra flags forwarded to every `cargo yank` invocation.
YANK_FLAGS ?=

# Pipeline that prints the workspace's publishable crate names, one per line.
# `--no-deps` keeps the list to workspace members so it stays correct as crates
# are added, merged, or removed.
list_crates = cargo metadata --no-deps --format-version 1 | tr '{' '\n' \
	| grep -oE '"name":"[^"]+","version":"[^"]+"' \
	| sed -E 's/"name":"([^"]+)".*/\1/' | sort -u

# The single workspace version, read from [workspace.package] in this Cargo.toml.
CRATE_VERSION := $(shell sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' Cargo.toml | head -1)

# Crates listed in dependency order (dependencies before dependents) so each is
# already on the registry when its dependents are published. Keep this in sync
# when adding/removing workspace members.
PUBLISH_CRATES ?= \
	doido-core \
	doido-controller-macros \
	doido-jobs-macros \
	doido-mailer-macros \
	doido-cable-macros \
	doido-model \
	doido-cache \
	doido-view \
	doido-jobs \
	doido-cable \
	doido-controller \
	doido-mailer \
	doido-storage \
	doido-generators \
	doido

.PHONY: help publish publish-dry-run clean-package check supply-chain yank unyank \
        fmt test verify example install-check services-up services-down test-backends \
        coverage coverage-check \
        blog blog-build blog-install

# Minimum line coverage (percent) required per workspace crate.
COVERAGE_THRESHOLD ?= 80

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# Wipe the isolated packaging target so each run starts from a clean slate.
clean-package: ## Remove the isolated packaging target dir
	rm -rf "$(PUBLISH_TARGET_DIR)"

publish-dry-run: clean-package ## Validate the whole workspace without uploading
	@command -v cargo >/dev/null || { echo "error: cargo not found in PATH" >&2; exit 1; }
	# Native workspace publish (cargo >= 1.90) packages every member up front and
	# resolves inter-crate deps within the batch, so unpublished members are
	# satisfied from the local package set instead of the live crates.io index.
	CARGO_TARGET_DIR="$(PUBLISH_TARGET_DIR)" cargo publish --workspace --dry-run $(PUBLISH_FLAGS)

publish: clean-package ## Upload the workspace to crates.io (resumable, rate-limit aware)
	@command -v cargo >/dev/null || { echo "error: cargo not found in PATH" >&2; exit 1; }
	@test -n "$(CRATE_VERSION)" || { echo "error: could not read workspace version from Cargo.toml" >&2; exit 1; }
	@echo "==> publishing workspace at version $(CRATE_VERSION)"
	@for crate in $(PUBLISH_CRATES); do \
		pfx=$$(printf '%s' "$$crate" | cut -c1-2)/$$(printf '%s' "$$crate" | cut -c3-4); \
		if command -v curl >/dev/null 2>&1 && \
		   curl -sf "https://index.crates.io/$$pfx/$$crate" 2>/dev/null | grep -q '"vers":"$(CRATE_VERSION)"'; then \
			echo "==> $$crate $(CRATE_VERSION): already published, skipping"; \
			continue; \
		fi; \
		while :; do \
			echo "==> publishing $$crate $(CRATE_VERSION)"; \
			out=$$(CARGO_TARGET_DIR="$(PUBLISH_TARGET_DIR)" cargo publish --allow-dirty -p "$$crate" $(PUBLISH_FLAGS) 2>&1); \
			code=$$?; \
			printf '%s\n' "$$out"; \
			if [ $$code -eq 0 ]; then \
				echo "    published; sleeping $(PUBLISH_INTERVAL)s before the next crate"; \
				sleep $(PUBLISH_INTERVAL); \
				break; \
			fi; \
			if printf '%s' "$$out" | grep -qiE 'already (uploaded|exists)'; then \
				echo "    already published, skipping"; break; \
			fi; \
			if printf '%s' "$$out" | grep -qiE '429|too many requests'; then \
				retry=$$(printf '%s' "$$out" | grep -oiE 'try again after .*GMT' | head -1 | sed -E 's/.*[Aa]fter[[:space:]]*//'); \
				now=$$(date -u +%s); \
				ready_ts=$$(date -u -d "$$retry" +%s 2>/dev/null || echo ""); \
				if [ -n "$$ready_ts" ]; then delay=$$(( ready_ts - now + $(PUBLISH_RETRY_BUFFER) )); else delay=$(PUBLISH_RETRY_BUFFER); fi; \
				if [ "$$delay" -lt "$(PUBLISH_RETRY_BUFFER)" ]; then delay=$(PUBLISH_RETRY_BUFFER); fi; \
				echo "    crates.io rate limit hit; waiting $${delay}s (until $$retry) then retrying"; \
				sleep "$$delay"; \
				continue; \
			fi; \
			echo "    error: failed to publish $$crate" >&2; exit 1; \
		done; \
	done; \
	echo "==> done: all crates published at $(CRATE_VERSION)"

# crates.io has no hard delete for published versions; `cargo yank` is the
# supported way to pull a version. Yanked versions can no longer be selected by
# new dependency resolution, but already-published crates that depend on them
# keep working. Use `make unyank` to reverse it. Requires a crates.io token
# (`cargo login` or CARGO_REGISTRY_TOKEN).
yank: ## Yank a published version of every workspace crate (VERSION=x.y.z)
	@command -v cargo >/dev/null || { echo "error: cargo not found in PATH" >&2; exit 1; }
	@ver='$(VERSION)'; \
	if [ -z "$$ver" ]; then \
		ver=$$(cargo metadata --no-deps --format-version 1 | tr '{' '\n' \
			| grep -m1 -oE '"name":"doido-core","version":"[^"]+"' \
			| sed -E 's/.*"version":"([^"]+)"/\1/'); \
	fi; \
	if [ -z "$$ver" ]; then echo "error: could not determine VERSION; pass VERSION=x.y.z" >&2; exit 1; fi; \
	for crate in $$($(list_crates)); do \
		echo "==> cargo yank $(YANK_FLAGS) $$crate@$$ver"; \
		cargo yank --version $$ver $(YANK_FLAGS) $$crate || exit $$?; \
	done

# Reverse a yank by re-running the `yank` recipe with --undo appended.
unyank: YANK_FLAGS += --undo
unyank: ## Restore (un-yank) a previously yanked version (VERSION=x.y.z)
unyank: yank

# ---------------------------------------------------------------------------
# Development & harness targets.
#
# `make verify` is the single green gate the (autonomous) harness relies on: it
# must exit 0 on a clean checkout. It chains the lint gate, the test suite, and
# — once it exists — the end-to-end example app. Keep these mirrored with the CI
# lint/test jobs in .github/workflows/ci.yml.
# ---------------------------------------------------------------------------

fmt: ## Format the whole workspace
	cargo fmt --all

check: ## Deterministic code gate: rustfmt + clippy (mirrors CI lint job)
	cargo fmt --check --all
	cargo clippy --workspace -- -D warnings

# Supply-chain audit is deliberately kept OUT of `verify`: the RustSec advisory
# database changes over time, so a newly published advisory in a transitive dep
# could turn the harness gate red with no code change. Run it in CI and on demand.
supply-chain: ## Supply-chain audit: cargo-deny + cargo-audit
	@if command -v cargo-deny  >/dev/null 2>&1; then cargo deny check; else echo "  (skip) cargo-deny not installed"; fi
	@if command -v cargo-audit >/dev/null 2>&1; then cargo audit;      else echo "  (skip) cargo-audit not installed"; fi

test: ## Run the workspace test suite (in-memory backends only)
	cargo test --workspace

coverage: ## Generate workspace line-coverage summary (requires cargo-llvm-cov)
	@command -v cargo-llvm-cov >/dev/null || { echo "error: cargo-llvm-cov not found; install with: cargo install cargo-llvm-cov" >&2; exit 1; }
	cargo llvm-cov --workspace --summary-only

coverage-check: ## Fail if any workspace crate is below COVERAGE_THRESHOLD (default 80%)
	COVERAGE_THRESHOLD=$(COVERAGE_THRESHOLD) ./scripts/coverage-check.sh

# End-to-end proof that `doido new` scaffolds a compiling app (the framework's
# definition-of-done). It builds the whole framework, so — like supply-chain — it
# is kept OUT of `verify`: a ~3min build must not gate the fast loop. Run in CI
# and on demand. The test itself is #[ignore]d.
example: ## Slow e2e: generate apps in a tempdir, compile them, and serve CRUD
	cargo test -p doido-generators --test e2e_app_build_test --test e2e_app_runtime_test -- --ignored --nocapture

# Release installer harness: build a local binary, curl-install it, run doido --help.
install-check: ## Validate scripts/install.sh (+ static checks for install.ps1)
	./scripts/verify-install.sh
	./scripts/verify-install-ps1.sh

# Coverage gate: every non-macro workspace crate must meet COVERAGE_THRESHOLD.
verify: check test coverage-check install-check ## Lint + tests + coverage + installer harness
	@echo "==> verify: OK"

# ---------------------------------------------------------------------------
# Backend services for feature-gated tests (postgres / redis / memcache).
# ---------------------------------------------------------------------------
services-up: ## Start dev backends (postgres, redis, memcache) via docker compose
	docker compose up -d

services-down: ## Stop and remove dev backends
	docker compose down -v

test-backends: ## Run feature-gated backend tests (needs `make services-up`)
	REDIS_URL=$${REDIS_URL:-redis://127.0.0.1:6379/} cargo test -p doido-jobs --features jobs-db,jobs-redis
	cargo test -p doido-cache --features cache-redis,cache-memcache

# ---------------------------------------------------------------------------
# Documentation + blog site (docs-blog/ — a Zola static site).
#
# `make blog` serves it locally with live reload. Zola is a single static
# binary: if it is not already on PATH, it is downloaded once into
# target/tools/ (Linux) so the target works out of the box. Keep ZOLA_VERSION
# in sync with .github/workflows/docs-blog.yml.
# ---------------------------------------------------------------------------
ZOLA_VERSION ?= 0.19.2
ZOLA_BIN     ?= $(CURDIR)/target/tools/zola
# Resolve a usable zola at recipe time: one on PATH, else the vendored binary.
zola_bin      = $$(command -v zola 2>/dev/null || echo "$(ZOLA_BIN)")

blog-install: ## Ensure Zola is available (downloads it into target/tools if missing)
	@if command -v zola >/dev/null 2>&1; then \
		echo "==> using system zola: $$(zola --version)"; \
	elif [ -x "$(ZOLA_BIN)" ]; then \
		echo "==> using vendored zola: $$($(ZOLA_BIN) --version)"; \
	else \
		os=$$(uname -s); arch=$$(uname -m); \
		if [ "$$os" != "Linux" ]; then \
			echo "error: zola not found. Install it (macOS: brew install zola) or see" >&2; \
			echo "       https://www.getzola.org/documentation/getting-started/installation/" >&2; \
			exit 1; \
		fi; \
		url="https://github.com/getzola/zola/releases/download/v$(ZOLA_VERSION)/zola-v$(ZOLA_VERSION)-$$arch-unknown-linux-gnu.tar.gz"; \
		echo "==> downloading zola $(ZOLA_VERSION) for $$arch-unknown-linux-gnu"; \
		mkdir -p "$(dir $(ZOLA_BIN))"; \
		curl -sfL "$$url" | tar xz -C "$(dir $(ZOLA_BIN))" zola || { echo "error: failed to download zola from $$url" >&2; exit 1; }; \
		echo "==> installed $$($(ZOLA_BIN) --version) at $(ZOLA_BIN)"; \
	fi

blog: blog-install ## Serve the docs + blog site locally at http://127.0.0.1:1111
	cd docs-blog && $(zola_bin) serve

blog-build: blog-install ## Build the docs + blog site into docs-blog/public
	cd docs-blog && $(zola_bin) build
