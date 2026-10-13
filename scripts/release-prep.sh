#!/usr/bin/env bash
# Prepare a release on a `release/X.Y.Z` branch: set the workspace version, stamp
# every crate's CHANGELOG, and commit the result so the tree matches the tag that
# the Release workflow will create.
#
# Usage:  scripts/release-prep.sh X.Y.Z
#         make release-prep VERSION=X.Y.Z
#
# What it does, in order:
#   1. Validates X.Y.Z is exact semver (no leading v).
#   2. Runs scripts/bump-workspace-version.sh — `cargo workspaces version --exact`
#      rewrites [workspace.package].version and every first-party dependency
#      requirement across all member manifests.
#   3. Stamps each published crate's CHANGELOG.md: renames `## Unreleased` to
#      `## X.Y.Z - <date>` and inserts a fresh empty `## Unreleased` above it, so the
#      accumulated bullets move under the new version heading.
#   4. Commits everything as `chore(release): X.Y.Z` (unless RELEASE_PREP_NO_COMMIT=1).
#
# NOTE: generator templates/tests still hardcode a version string
# (doido-generators/src/project_auth.rs, src/generators/new.rs). Making the
# generator derive its version dynamically is tracked as a separate change; this
# script deliberately only touches Cargo manifests (via cargo-workspaces) + changelogs.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-${VERSION:-}}"
: "${VERSION:?usage: scripts/release-prep.sh X.Y.Z (exact semver, no leading v)}"

if ! printf '%s' "${VERSION}" \
	| grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'; then
	echo "error: VERSION must be exact semver with no leading v (e.g. 0.1.0); got '${VERSION}'" >&2
	exit 1
fi

branch="$(git rev-parse --abbrev-ref HEAD)"
case "$branch" in
release/*) : ;;
*) echo "warning: not on a release/* branch (on '${branch}'); releases publish only from release/* — continuing anyway" >&2 ;;
esac

DATE="$(date -u +%F)"

echo "==> setting workspace version to ${VERSION}"
VERSION="${VERSION}" "${ROOT}/scripts/bump-workspace-version.sh" >/dev/null

echo "==> stamping changelogs (## Unreleased -> ## ${VERSION} - ${DATE})"
# Iterate the published workspace crates so only real crate changelogs are stamped.
cargo metadata --no-deps --format-version 1 \
	| tr ',' '\n' | grep -oE '"manifest_path":"[^"]+"' \
	| sed -E 's/"manifest_path":"([^"]+)\/Cargo.toml"/\1/' | sort -u \
	| while read -r dir; do
		cl="${dir}/CHANGELOG.md"
		[ -f "$cl" ] || { echo "    warning: no CHANGELOG.md in ${dir}, skipping" >&2; continue; }
		if ! grep -qxF '## Unreleased' "$cl"; then
			echo "    warning: no '## Unreleased' heading in ${cl}, skipping" >&2
			continue
		fi
		awk -v ver="$VERSION" -v date="$DATE" '
			!done && $0 == "## Unreleased" {
				print "## Unreleased"
				print ""
				print "_No user-facing changes yet._"
				print ""
				print "## " ver " - " date
				done = 1
				next
			}
			{ print }
		' "$cl" >"${cl}.tmp"
		mv "${cl}.tmp" "$cl"
		echo "    stamped ${cl}"
	done

if [ "${RELEASE_PREP_NO_COMMIT:-0}" = "1" ]; then
	echo "==> RELEASE_PREP_NO_COMMIT=1 set; leaving changes uncommitted"
	exit 0
fi

git add -A
git commit -m "chore(release): ${VERSION}" >/dev/null
echo "==> committed release prep for ${VERSION} on ${branch}"
echo "    next: push, run 'make verify', then dispatch the Release workflow from this branch"
