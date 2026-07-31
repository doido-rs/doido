#!/usr/bin/env bash
# Static checks for scripts/install.ps1 (syntax + required symbols).
# Full end-to-end install is exercised on windows-latest in release.yml.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="${ROOT}/scripts/install.ps1"

[ -f "$SCRIPT" ] || {
	echo "error: missing ${SCRIPT}" >&2
	exit 1
}

required=(
	'DOIDO_VERSION'
	'DOIDO_INSTALL_DIR'
	'DOIDO_DOWNLOAD_BASE'
	'x86_64-pc-windows-msvc'
	'Get-LatestVersion'
	'Invoke-WebRequest'
)

for sym in "${required[@]}"; do
	grep -q "$sym" "$SCRIPT" || {
		echo "error: install.ps1 missing expected content: ${sym}" >&2
		exit 1
	}
done

if command -v pwsh >/dev/null 2>&1; then
	pwsh -NoProfile -Command "& { \$null = [System.Management.Automation.Language.Parser]::ParseFile('${SCRIPT}', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs | ForEach-Object { Write-Error \$_ }; exit 1 } }"
elif command -v powershell >/dev/null 2>&1; then
	powershell -NoProfile -Command "& { \$null = [System.Management.Automation.Language.Parser]::ParseFile('${SCRIPT}', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs | ForEach-Object { Write-Error \$_ }; exit 1 } }"
else
	echo "warning: PowerShell not found; skipped syntax parse (static grep OK)"
fi

echo "==> verify-install-ps1: OK"
