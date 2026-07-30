#!/usr/bin/env bash
# Bump the workspace version with cargo-workspaces and print the resolved semver.
#
# Environment:
#   VERSION_TYPE  major | minor | patch | premajor | preminor | prepatch |
#                 skip | prerelease | custom
#   VERSION       Required when VERSION_TYPE=custom (exact semver, no leading v)
#
# When GITHUB_OUTPUT is set (GitHub Actions), writes `version=<semver>` there too.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION_TYPE="${VERSION_TYPE:?VERSION_TYPE is required}"
VERSION="${VERSION:-}"

read_workspace_version() {
	"${ROOT}/scripts/read-workspace-version.sh"
}

write_output() {
	local resolved="$1"
	echo "==> release version: ${resolved}"
	if [ -n "${GITHUB_OUTPUT:-}" ]; then
		echo "version=${resolved}" >>"${GITHUB_OUTPUT}"
	fi
}

case "${VERSION_TYPE}" in
custom)
	[ -n "${VERSION}" ] || {
		echo "error: VERSION is required when VERSION_TYPE=custom" >&2
		exit 1
	}
	cargo workspaces version --no-git-commit --exact --yes custom "${VERSION}"
	write_output "${VERSION}"
	;;
skip)
	write_output "$(read_workspace_version)"
	;;
major | minor | patch | premajor | preminor | prepatch | prerelease)
	cargo workspaces version --no-git-commit --exact --yes "${VERSION_TYPE}"
	write_output "$(read_workspace_version)"
	;;
*)
	echo "error: unsupported VERSION_TYPE: ${VERSION_TYPE}" >&2
	echo "expected: major, minor, patch, premajor, preminor, prepatch, skip, prerelease, custom" >&2
	exit 1
	;;
esac
