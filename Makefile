# Doido workspace Makefile
#
# `make publish` releases the whole workspace to crates.io in a single
# `cargo publish --workspace` run. Cargo computes the dependency order itself,
# packages every crate up front, and resolves the inter-crate dependencies
# within the batch — so it never blocks waiting for one crate to be indexed
# before publishing the next.

# Extra flags forwarded to every `cargo publish` invocation
# (e.g. `make publish PUBLISH_FLAGS=--no-verify`).
PUBLISH_FLAGS ?=

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

.PHONY: help publish publish-dry-run check yank unyank

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

publish: ## Publish the whole workspace to crates.io
	@command -v cargo >/dev/null || { echo "error: cargo not found in PATH" >&2; exit 1; }
	cargo publish --workspace $(PUBLISH_FLAGS)

publish-dry-run: ## Package and verify the workspace without uploading
	cargo publish --workspace --dry-run $(PUBLISH_FLAGS)

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
