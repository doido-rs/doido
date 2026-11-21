#!/usr/bin/env bash
# Block until every crate listed before TARGET in scripts/publish-crates.txt is
# visible on the crates.io index at VERSION. No-op when TARGET is first in the list.
#
# Used by `make publish` so a dependent crate is not packaged while a just-published
# dependency is still propagating (otherwise cargo hits crates.io and fails with
# "failed to select a version").
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIST="${ROOT}/scripts/publish-crates.txt"
VERSION="${1:?usage: wait-publish-deps.sh VERSION TARGET}"
TARGET="${2:?usage: wait-publish-deps.sh VERSION TARGET}"

POLL_INTERVAL="${PUBLISH_INDEX_POLL_INTERVAL:-15}"
MAX_WAIT="${PUBLISH_INDEX_MAX_WAIT:-900}"

index_prefix() {
	local crate="$1"
	printf '%s/%s' "$(printf '%s' "$crate" | cut -c1-2)" "$(printf '%s' "$crate" | cut -c3-4)"
}

on_index() {
	local crate="$1"
	local pfx
	pfx="$(index_prefix "$crate")"
	command -v curl >/dev/null 2>&1 \
		&& curl -sf "https://index.crates.io/${pfx}/${crate}" 2>/dev/null \
		| grep -q "\"vers\":\"${VERSION}\""
}

priors=()
while IFS= read -r line || [ -n "$line" ]; do
	line="${line%%#*}"
	line="$(printf '%s' "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
	[ -z "$line" ] && continue
	[ "$line" = "$TARGET" ] && break
	priors+=("$line")
done <"$LIST"

for dep in "${priors[@]}"; do
	elapsed=0
	while ! on_index "$dep"; do
		if [ "$elapsed" -ge "$MAX_WAIT" ]; then
			echo "error: ${dep} ${VERSION} not on crates.io index after ${MAX_WAIT}s" >&2
			exit 1
		fi
		if [ "$elapsed" -eq 0 ]; then
			echo "    waiting for ${dep} ${VERSION} on crates.io index..."
		fi
		sleep "$POLL_INTERVAL"
		elapsed=$((elapsed + POLL_INTERVAL))
	done
done
