#!/usr/bin/env bash
# Ensure scripts/publish-crates.txt lists every workspace member exactly once.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIST="${ROOT}/scripts/publish-crates.txt"

mapfile -t WORKSPACE < <(
	cargo metadata --no-deps --format-version 1 \
		| python3 -c 'import json,sys; print("\n".join(sorted(p["name"] for p in json.load(sys.stdin)["packages"])))'
)

mapfile -t PUBLISH < <(grep -vE '^\s*(#|$)' "$LIST")

if ((${#WORKSPACE[@]} != ${#PUBLISH[@]})); then
	echo "error: publish list has ${#PUBLISH[@]} crate(s), workspace has ${#WORKSPACE[@]}" >&2
fi

missing=()
extra=()
for pkg in "${WORKSPACE[@]}"; do
	if ! printf '%s\n' "${PUBLISH[@]}" | grep -qx "$pkg"; then
		missing+=("$pkg")
	fi
done
for pkg in "${PUBLISH[@]}"; do
	if ! printf '%s\n' "${WORKSPACE[@]}" | grep -qx "$pkg"; then
		extra+=("$pkg")
	fi
done

if ((${#missing[@]} > 0)); then
	echo "error: workspace members missing from scripts/publish-crates.txt:" >&2
	printf '  - %s\n' "${missing[@]}" >&2
fi
if ((${#extra[@]} > 0)); then
	echo "error: scripts/publish-crates.txt lists non-workspace crates:" >&2
	printf '  - %s\n' "${extra[@]}" >&2
fi

if ((${#missing[@]} > 0 || ${#extra[@]} > 0)); then
	exit 1
fi

echo "==> publish order: ${#PUBLISH[@]} crates (matches workspace members)"
