# Install the `doido` CLI from a GitHub Release binary.
#
# Usage (PowerShell):
#   irm https://github.com/doido-rs/doido/releases/latest/download/install.ps1 | iex
#   $env:DOIDO_VERSION = "0.0.9"; irm .../install.ps1 | iex
#
# Environment:
#   DOIDO_VERSION       Semver without leading "v" (default: latest release)
#   DOIDO_INSTALL_DIR   Install directory (default: $HOME\.local\bin)
#   DOIDO_GITHUB_REPO   owner/repo override (default: doido-rs/doido)
#   DOIDO_DOWNLOAD_BASE Override download base URL (for local/harness testing)

$ErrorActionPreference = "Stop"

$Repo = if ($env:DOIDO_GITHUB_REPO) { $env:DOIDO_GITHUB_REPO } else { "doido-rs/doido" }
$InstallDir = if ($env:DOIDO_INSTALL_DIR) { $env:DOIDO_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
$Version = $env:DOIDO_VERSION
$DownloadBase = $env:DOIDO_DOWNLOAD_BASE

function Write-Info([string]$Message) {
    Write-Host "==> $Message"
}

function Write-Warn([string]$Message) {
    Write-Warning $Message
}

function Get-DoidoTarget {
    if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
            [System.Runtime.InteropServices.OSPlatform]::Windows)) {
        throw "unsupported operating system (use scripts/install.sh on Linux/macOS)"
    }

    switch ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture) {
        "X64" { return "x86_64-pc-windows-msvc" }
        "Arm64" { throw "unsupported Windows architecture: Arm64 (no release binary yet)" }
        default { throw "unsupported Windows architecture: $([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture)" }
    }
}

function Get-LatestVersion {
    $uri = "https://api.github.com/repos/$Repo/releases/latest"
    $release = Invoke-RestMethod -Uri $uri -Headers @{ "User-Agent" = "doido-installer" }
    return ($release.tag_name -replace '^v', '')
}

function Get-DownloadUrl([string]$Target, [string]$ReleaseVersion) {
    if ($DownloadBase) {
        return "$DownloadBase/doido-$Target.exe"
    }
    return "https://github.com/$Repo/releases/download/v$ReleaseVersion/doido-$Target.exe"
}

function Ensure-PathEntry([string]$Directory) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if ($userPath) { $parts = $userPath -split ';' | Where-Object { $_ -ne "" } }

    $normalized = $Directory.TrimEnd('\')
    if ($parts -notcontains $normalized) {
        $parts += $normalized
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ';'), "User")
        $env:Path = "$normalized;$env:Path"
        Write-Warn "added $normalized to your user PATH (open a new terminal if doido is not found)"
    }
}

function Install-Doido {
    $target = Get-DoidoTarget
    Write-Info "detected target: $target"

    if (-not $Version) {
        if ($DownloadBase) {
            throw "DOIDO_DOWNLOAD_BASE is set; you must also set DOIDO_VERSION"
        }
        Write-Info "resolving latest release"
        $Version = Get-LatestVersion
    }

    Write-Info "installing doido $Version"

    $url = Get-DownloadUrl -Target $target -ReleaseVersion $Version
    $dest = Join-Path $InstallDir "doido.exe"

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    Write-Info "downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing

    Ensure-PathEntry $InstallDir

    Write-Info "installed $dest"
    & $dest --help | Out-Null
    Write-Info "doido is ready"
}

Install-Doido
