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
	doido-view \
	doido-model \
	doido-cache \
	doido-controller \
	doido-mailer \
	doido-jobs \
	doido-cable \
	doido-generators \
	doido

.PHONY: help publish publish-dry-run clean-package check yank unyank

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
			out=$$(CARGO_TARGET_DIR="$(PUBLISH_TARGET_DIR)" cargo publish -p "$$crate" $(PUBLISH_FLAGS) 2>&1); \
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

check: ## Run cargo deny supply-chain checks
	cargo deny check
