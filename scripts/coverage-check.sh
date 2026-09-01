#!/usr/bin/env bash
# Enforce line-coverage thresholds for every workspace crate.
#
# Usage:
#   scripts/coverage-check.sh              # crate-level gate (default)
#   scripts/coverage-check.sh --per-file   # also require every src file >= threshold
#
# Environment:
#   COVERAGE_THRESHOLD  Minimum line coverage percent (default: 80)
#   COVERAGE_PACKAGES   Space-separated package list (default: all workspace members)
#   COVERAGE_JOBS       Max parallel rustc jobs for the instrumented coverage
#                       builds (default: 3). Coverage builds carry -C
#                       instrument-coverage and use more RAM per rustc, so this
#                       caps below the normal build to avoid swap thrashing on
#                       low-memory machines.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

THRESHOLD="${COVERAGE_THRESHOLD:-80}"
PER_FILE=0
if [[ "${1:-}" == "--per-file" ]]; then
	PER_FILE=1
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
	echo "error: cargo-llvm-cov not found; install with: cargo install cargo-llvm-cov" >&2
	exit 1
fi

# Instrumented coverage builds are the heaviest workload here (extra
# -C instrument-coverage codegen + a test-binary run per crate). Cap parallel
# rustc jobs so the loop doesn't exhaust RAM and push the box into swap. Env
# takes precedence over the workspace-wide [build] jobs in .cargo/config.toml.
export CARGO_BUILD_JOBS="${COVERAGE_JOBS:-3}"

if [[ -n "${COVERAGE_PACKAGES:-}" ]]; then
	# shellcheck disable=SC2206
	PACKAGES=(${COVERAGE_PACKAGES})
else
	mapfile -t PACKAGES < <(
		cargo metadata --no-deps --format-version 1 \
			| python3 -c 'import json,sys; print("\n".join(sorted(
			    p for p in {pkg["name"] for pkg in json.load(sys.stdin)["packages"]}
			    if not p.endswith("-macros")
			)))'
	)
fi

# Per-crate coverage floors (the ratchet) from
# [workspace.metadata.coverage-gate.crates] in Cargo.toml. Crates listed there
# must meet their own floor; every other crate must meet the global THRESHOLD.
# This keeps the gate green while lagging crates are brought up to the 80%
# target one at a time — raise a crate's floor as its coverage rises, and drop
# it from the table once it clears THRESHOLD.
declare -A CRATE_FLOORS=()
while IFS=' ' read -r name floor; do
	[[ -z "$name" ]] && continue
	CRATE_FLOORS["$name"]="$floor"
done < <(
	cargo metadata --no-deps --format-version 1 2>/dev/null \
		| python3 -c '
import json, sys
meta = json.load(sys.stdin).get("metadata") or {}
crates = ((meta.get("coverage-gate") or {}).get("crates")) or {}
for name, floor in crates.items():
    print(f"{name} {floor}")
'
)

# Coverage floor for a crate: its per-crate ratchet floor if present, else the
# global THRESHOLD.
crate_floor() {
	if [[ -n "${CRATE_FLOORS[$1]:-}" ]]; then
		echo "${CRATE_FLOORS[$1]}"
	else
		echo "$THRESHOLD"
	fi
}

parse_crate_line_pct() {
	python3 -c '
import sys
for line in sys.stdin:
    if not line.startswith("TOTAL"):
        continue
    pcts = [p.rstrip("%") for p in line.split() if p.endswith("%")]
    if len(pcts) >= 3:
        print(pcts[2])
        break
'
}

parse_file_failures() {
	python3 - "$THRESHOLD" <<'PY'
import sys

threshold = float(sys.argv[1])
for line in sys.stdin.read().splitlines():
    if not line.startswith("doido-"):
        continue
    parts = line.split()
    pcts = [p for p in parts if p.endswith("%")]
    if len(pcts) < 3:
        continue
    file_line_pct = float(pcts[2].rstrip("%"))
    if file_line_pct < threshold:
        print(f"{parts[0]} {file_line_pct:.2f}")
PY
}

failed_crates=()
failed_files=()

# Extra flags for crates whose backends are feature-gated.
coverage_extra_args() {
	case "$1" in
	doido-jobs) echo "--features jobs-db,jobs-redis" ;;
	# schema_design and its tests require `cli`; without it, llvm-cov can
	# still attribute 0% lines from binaries built earlier by `doido`.
	doido-model) echo "--features cli" ;;
	esac
}

# Per-crate `--ignore-filename-regex` patterns: files that must not count toward
# a crate's line-coverage gate.
coverage_ignore_regex() {
	case "$1" in
	# server.rs boots the real HTTP server (DB pool + views + axum::serve);
	# it is covered end-to-end by the release e2e, not by in-process unit tests.
	doido) echo 'doido/src/server\.rs' ;;
	# commands/ is `#[cfg(feature = "cli")]` and covered by release e2e, not
	# unit tests. postgres/mysql introspect adapters are feature-gated and
	# not exercised in the sqlite coverage run. Without these exclusions,
	# llvm-cov can attribute 0% lines from binaries built earlier by `doido`.
	doido-model) echo 'doido-model/src/commands/|doido-model/src/schema_design/introspect/(postgres|mysql)\.rs' ;;
	# CLI boot/REPL/server dispatchers: exercised by the release e2e (#[ignore]),
	# not by in-process unit tests. The data-oriented commands (db/credentials/
	# jobs/worker/generate/destroy) stay in the gate and are unit-tested.
	doido-generators) echo 'doido-generators/src/(commands/(server|console|dbconsole|runner)|banner|cli|main)\.rs' ;;
	# commands/server.rs boots axum; commands/console.rs is an interactive REPL.
	doido-controller) echo 'doido-controller/src/commands/(server|console)\.rs' ;;
	esac
}

echo "==> coverage gate: line coverage >= per-crate floor (global default ${THRESHOLD}%, per-file=${PER_FILE})"
for pkg in "${PACKAGES[@]}"; do
	echo "    measuring ${pkg}..."
	extra="$(coverage_extra_args "$pkg")"
	ignore="$(coverage_ignore_regex "$pkg")"
	summary="$(cargo llvm-cov -p "$pkg" ${extra} ${ignore:+--ignore-filename-regex "$ignore"} --summary-only 2>/dev/null || true)"
	if [[ -z "$summary" ]]; then
		echo "error: no coverage summary for ${pkg}" >&2
		failed_crates+=("${pkg} (no summary)")
		continue
	fi

	total_line="$(printf '%s\n' "$summary" | parse_crate_line_pct)"
	if [[ -z "$total_line" ]]; then
		echo "error: could not parse TOTAL line for ${pkg}" >&2
		failed_crates+=("${pkg} (parse error)")
		continue
	fi

	floor="$(crate_floor "$pkg")"
	status="OK"
	if awk "BEGIN { exit !(${total_line} < ${floor}) }"; then
		status="FAIL"
		failed_crates+=("${pkg} ${total_line}% (floor ${floor}%)")
	fi
	printf '    %-24s %6.2f%%  (floor %5.1f%%)  [%s]\n' "${pkg}" "${total_line}" "${floor}" "${status}"

	if [[ "$PER_FILE" -eq 1 ]]; then
		while IFS= read -r row; do
			[[ -z "$row" ]] && continue
			failed_files+=("${row}%")
		done < <(printf '%s\n' "$summary" | parse_file_failures)
	fi
done

if ((${#failed_crates[@]} > 0)); then
	echo
	echo "==> crates below their coverage floor:"
	for entry in "${failed_crates[@]}"; do
		echo "    - ${entry}"
	done
fi

if ((${#failed_files[@]} > 0)); then
	echo
	echo "==> source files below ${THRESHOLD}%:"
	for entry in "${failed_files[@]}"; do
		echo "    - ${entry}"
	done
fi

if ((${#failed_crates[@]} > 0 || ${#failed_files[@]} > 0)); then
	exit 1
fi

echo
echo "==> coverage gate: OK (all crates >= their floor)"
