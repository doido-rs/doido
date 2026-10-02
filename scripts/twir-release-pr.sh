#!/usr/bin/env bash
# Open a pull request against This Week in Rust's current draft for a stable release.
#
#   bash scripts/twir-release-pr.sh <version>
#
# Run from the Release workflow after GitHub Release (stable only). Requires a token
# that can push to a fork of rust-lang/this-week-in-rust and open PRs upstream.
#
#   GH_TOKEN    classic PAT with public_repo (CI: TWIR_TOKEN secret)
#   TWIR_FORK   owner/name of the fork, default <login>/this-week-in-rust
#   DRY_RUN=1   edit a clone of upstream, print the diff, push and open nothing
#   DOIDO_GITHUB_REPO  owner/repo for release links (default doido-rs/doido)
#
# Idempotent: branch doido-v<version> on the fork, or URL already in draft → exit 0.
set -euo pipefail

version="${1:?usage: twir-release-pr.sh <version>}"
section="${TWIR_SECTION:-Observations/Thoughts}"
upstream="rust-lang/this-week-in-rust"
repo="${DOIDO_GITHUB_REPO:-doido-rs/doido}"
title="Doido v${version}"
url="https://github.com/${repo}/releases/tag/v${version}"
branch="doido-v${version}"

root="$(cd "$(dirname "$0")/.." && pwd)"
py="$(command -v python3 || command -v python)" || {
	echo "python is required" >&2
	exit 1
}

token="${GH_TOKEN:-$(gh auth token 2>/dev/null || true)}"
[ -n "$token" ] || {
	echo "no GitHub token: set GH_TOKEN (or TWIR_TOKEN in CI)" >&2
	exit 1
}
export GH_TOKEN="$token"
login="$(gh api user --jq .login)"
fork="${TWIR_FORK:-$login/this-week-in-rust}"
base="$(gh api "repos/$upstream" --jq .default_branch)"
dry="${DRY_RUN:-0}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if [ "$dry" = 1 ]; then
	echo "dry run: editing a clone of $upstream, pushing nothing"
	git -c core.autocrlf=false clone --quiet --filter=blob:none --branch "$base" \
		"https://github.com/$upstream.git" "$tmp/twir"
else
	gh repo fork "$upstream" --clone=false >/dev/null 2>&1 || true
	gh repo sync "$fork" --source "$upstream" --branch "$base" --force >/dev/null
	if gh api "repos/$fork/branches/$branch" >/dev/null 2>&1; then
		echo "$fork already has branch $branch; the PR exists or was closed. Nothing to do."
		exit 0
	fi
	git -c core.autocrlf=false clone --quiet --filter=blob:none --branch "$base" \
		"https://github.com/$fork.git" "$tmp/twir"
fi
cd "$tmp/twir"

drafts=()
for f in draft/*-this-week-in-rust.md; do
	[ -f "$f" ] || continue
	drafts+=("$f")
done
if [ "${#drafts[@]}" -ne 1 ]; then
	echo "expected exactly one draft/*-this-week-in-rust.md in $upstream, found: ${drafts[*]:-none}. Re-run later." >&2
	exit 1
fi
draft="${drafts[0]}"

set +e
"$py" "$root/scripts/twir-draft-insert.py" "$draft" "$title" "$url" "$section"
rc=$?
set -e
case "$rc" in
0) ;;
3) echo "Nothing to do."; exit 0 ;;
*) exit "$rc" ;;
esac

git checkout --quiet -b "$branch"
git -c user.name="github-actions[bot]" \
	-c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
	commit --quiet -am "Add: $title"

if [ "$dry" = 1 ]; then
	git --no-pager show --stat --format='%s' HEAD
	git --no-pager diff HEAD~1 -- "$draft"
	exit 0
fi

changelog_blurb=""
if [ -f "$root/CHANGELOG.md" ]; then
	changelog_blurb="$(
		"$py" - "$root/CHANGELOG.md" "$version" <<'EOF'
import re
import sys

path, version = sys.argv[1:3]
text = open(path, encoding="utf-8").read()
heading = f"## [{version}]"
start = text.find(heading)
if start < 0:
    sys.exit(0)
rest = text[start + len(heading) :]
end = rest.find("\n## ")
block = rest[:end] if end >= 0 else rest
lines = []
for line in block.splitlines():
    s = line.strip()
    if not s or s.startswith("#"):
        continue
    if s.startswith("###"):
        continue
    lines.append(s)
    if len(lines) >= 3:
        break
if lines:
    print("\n".join(lines))
EOF
	)" || true
fi

git push --quiet "https://x-access-token:${token}@github.com/$fork.git" "HEAD:$branch"

body="$url

Automated submission from the Doido stable release workflow."
if [ -n "$changelog_blurb" ]; then
	body="$body

Release notes excerpt:

$changelog_blurb"
fi

gh pr create --repo "$upstream" --base "$base" --head "${fork%%/*}:$branch" \
	--title "Release: $title" --body "$body"
