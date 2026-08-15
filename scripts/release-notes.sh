#!/usr/bin/env bash
# Assemble the GitHub Release body for a version by concatenating each published
# crate's CHANGELOG.md section for that version, in publish order. Crates whose
# only entry is the "no user-facing changes" placeholder are omitted so the notes
# stay signal-heavy.
#
# Usage:  scripts/release-notes.sh X.Y.Z            # writes to stdout
#         scripts/release-notes.sh X.Y.Z NOTES.md   # also writes to NOTES.md
#
# Output is Markdown: a per-crate "### <crate>" heading + that crate's section
# body. The Release workflow appends install instructions.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-}"
: "${VERSION:?usage: scripts/release-notes.sh X.Y.Z [outfile]}"
OUT="${2:-}"

# Build crate-name -> directory from the workspace manifests (one pass).
declare -A CRATE_DIR
while IFS= read -r manifest; do
	name="$(sed -nE 's/^name = "([^"]+)".*/\1/p' "$manifest" | head -1)"
	[ -n "$name" ] && CRATE_DIR["$name"]="$(dirname "$manifest")"
done < <(find . -name Cargo.toml -not -path '*/target/*' -not -path '*/target*/*')

# Extract the section body under "## <version>" up to the next "## " heading,
# with leading/trailing blank lines trimmed.
section_body() {
	awk -v ver="$1" '
		$0 ~ "^## " ver "( |$)" { grab = 1; next }
		grab && /^## / { exit }
		grab { print }
	' "$2" | awk 'NF {p=1} p' | tac | awk 'NF {p=1} p' | tac
}

emit() {
	printf "## What's changed in %s\n\n" "$VERSION"
	# Publish order (scripts/publish-crates.txt), comments/blanks stripped.
	while IFS= read -r crate; do
		[ -n "$crate" ] || continue
		dir="${CRATE_DIR[$crate]:-}"
		[ -n "$dir" ] || continue
		cl="${dir}/CHANGELOG.md"
		[ -f "$cl" ] || continue
		body="$(section_body "$VERSION" "$cl")"
		[ -n "$body" ] || continue
		# Skip crates whose section is just the "no user-facing changes" placeholder.
		if printf '%s' "$body" | grep -qiE 'no user-facing changes' \
			&& ! printf '%s\n' "$body" | grep -qE '^[-*] '; then
			continue
		fi
		printf '### %s\n\n%s\n\n' "$crate" "$body"
	done < <(sed -E '/^[[:space:]]*(#|$)/d' scripts/publish-crates.txt)
}

notes="$(emit)"
printf '%s\n' "$notes"
[ -n "$OUT" ] && printf '%s\n' "$notes" >"$OUT"
