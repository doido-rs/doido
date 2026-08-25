#!/usr/bin/env bash
# Set the workspace to an exact version with cargo-workspaces and print it.
#
# The release publishes exactly the version supplied here — there is no keyword
# bumping. `cargo workspaces version --exact custom` rewrites both
# [workspace.package].version and every first-party [workspace.dependencies]
# entry, which is what `make publish` reads back as CRATE_VERSION.
#
# Environment:
#   VERSION   Required. Exact semver to publish (no leading v), e.g. 0.1.0
#   UNSTABLE  When 1/true/yes, VERSION must include a pre-release suffix (e.g. 0.0.25-test.1)
#
# When GITHUB_OUTPUT is set (GitHub Actions), writes `version=<semver>` there too.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${VERSION:?VERSION is required (exact semver, no leading v, e.g. 0.1.0)}"

# Guard against a leading v or a typo publishing a garbage tag/crate version.
if ! printf '%s' "${VERSION}" \
	| grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'; then
	echo "error: VERSION must be an exact semver with no leading v (e.g. 0.1.0); got '${VERSION}'" >&2
	exit 1
fi

case "${UNSTABLE:-}" in
1 | true | yes | TRUE | YES)
	if ! printf '%s' "${VERSION}" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+-'; then
		echo "error: UNSTABLE release requires a pre-release suffix (e.g. 0.0.25-test.1); got '${VERSION}'" >&2
		exit 1
	fi
	;;
esac

cargo workspaces version --no-git-commit --exact --yes custom "${VERSION}"

echo "==> release version: ${VERSION}"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
	echo "version=${VERSION}" >>"${GITHUB_OUTPUT}"
fi
