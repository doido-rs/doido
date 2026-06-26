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

.PHONY: help publish publish-dry-run check

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

publish: ## Publish the whole workspace to crates.io
	@command -v cargo >/dev/null || { echo "error: cargo not found in PATH" >&2; exit 1; }
	cargo publish --workspace $(PUBLISH_FLAGS)

publish-dry-run: ## Package and verify the workspace without uploading
	cargo publish --workspace --dry-run $(PUBLISH_FLAGS)

check: ## Run cargo deny supply-chain checks
	cargo deny check
