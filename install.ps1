# agentverse installer script for Windows
#
# Usage:
#   irm https://raw.githubusercontent.com/loonghao/agentverse/main/install.ps1 | iex
#
# With specific version:
#   $env:AGENTVERSE_VERSION="0.1.3"; irm https://raw.githubusercontent.com/loonghao/agentverse/main/install.ps1 | iex
#
# With custom install directory:
#   $env:AGENTVERSE_INSTALL_DIR="C:\tools\bin"; irm https://raw.githubusercontent.com/loonghao/agentverse/main/install.ps1 | iex

param(
    [string]$Version    = $env:AGENTVERSE_VERSION,
    [string]$InstallDir = $env:AGENTVERSE_INSTALL_DIR
)

$ErrorActionPreference = "Stop"

$RepoOwner = "loonghao"
$RepoName  = "agentverse"
$BaseUrl   = "https://github.com/$RepoOwner/$RepoName/releases"

if (-not $InstallDir) {
    $InstallDir = "$env:USERPROFILE\.local\bin"
}

# ── Logging ──────────────────────────────────────────────────────────────────

function Write-Step { param([string]$m) Write-Host "  agentverse " -NoNewline -ForegroundColor Cyan;  Write-Host $m }
function Write-Ok   { param([string]$m) Write-Host "  agentverse " -NoNewline -ForegroundColor Green; Write-Host $m }
function Write-Fail { param([string]$m) Write-Host "  agentverse " -NoNewline -ForegroundColor Red;   Write-Host $m; exit 1 }

# ── Platform detection ────────────────────────────────────────────────────────

function Get-Platform {
    $arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture) {
        "X64"   { "x86_64" }
        "Arm64" { "aarch64" }
        default { if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" } }
    }
    return "$arch-pc-windows-msvc"
}

# ── Download with retry ───────────────────────────────────────────────────────

function Invoke-Download {
    param([string]$Url, [string]$Dest)

    $headers = @{ "User-Agent" = "agentverse-installer/1.0" }
    if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $env:GITHUB_TOKEN" }

    for ($i = 1; $i -le 3; $i++) {
        try {
            $wc = New-Object System.Net.WebClient
            foreach ($k in $headers.Keys) { $wc.Headers.Add($k, $headers[$k]) }
            $wc.DownloadFile($Url, $Dest)
            if ((Test-Path $Dest) -and (Get-Item $Dest).Length -gt 1024) { return $true }
        } catch {
            if ($i -lt 3) { Start-Sleep -Seconds 2 }
        }
        Remove-Item $Dest -Force -ErrorAction SilentlyContinue
    }
    return $false
}

# ── Resolve latest GitHub release ─────────────────────────────────────────────

function Get-LatestVersion {
    $headers = @{
        "User-Agent" = "agentverse-installer/1.0"
        "Accept"     = "application/vnd.github+json"
    }
    if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $env:GITHUB_TOKEN" }

    try {
        $apiUrl   = "https://api.github.com/repos/$RepoOwner/$RepoName/releases?per_page=10"
        $releases = Invoke-RestMethod -Uri $apiUrl -Headers $headers -TimeoutSec 30
        foreach ($r in $releases) {
            if (-not $r.prerelease -and -not $r.draft -and $r.assets -and $r.assets.Count -gt 0) {
                return ($r.tag_name -replace '^v', '')
            }
        }
    } catch { }
    return $null
}

# ── Main ──────────────────────────────────────────────────────────────────────

function Main {
    $platform = Get-Platform

    Write-Step "Installing agentverse CLI for Windows..."
    Write-Step "Detected: Windows -> $platform"

    # Resolve version
    $ver = $Version -replace '^v', ''
    if (-not $ver -or $ver -eq "latest") {
        $ver = Get-LatestVersion
        if (-not $ver) { Write-Fail "Could not resolve latest version. Set AGENTVERSE_VERSION explicitly." }
        Write-Step "Resolved latest version: $ver"
    }

    # Download candidates
    $candidates = @(
        "$BaseUrl/download/v$ver/agentverse-$ver-$platform.zip",
        "$BaseUrl/download/v$ver/agentverse-$platform.zip",
        "$BaseUrl/latest/download/agentverse-$platform.zip"
    )

    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }

    try {
        $archivePath = $null

        foreach ($url in $candidates) {
            Write-Step "Trying: $url"
            $dest = Join-Path $tempDir ($url.Split('/')[-1])
            if (Invoke-Download -Url $url -Dest $dest) {
                $archivePath = $dest
                break
            }
        }

        if (-not $archivePath) {
            Write-Fail "Download failed. Try: `$env:AGENTVERSE_VERSION='$ver'; irm https://raw.githubusercontent.com/$RepoOwner/$RepoName/main/install.ps1 | iex"
        }

        Write-Step "Extracting..."
        if (-not (Test-Path $InstallDir)) { New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null }
        Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force

        $binary = Get-ChildItem -Path $tempDir -Filter "agentverse.exe" -Recurse | Select-Object -First 1
        if (-not $binary) { Write-Fail "agentverse.exe not found in archive" }

        $destBin = Join-Path $InstallDir "agentverse.exe"
        Copy-Item -Path $binary.FullName -Destination $destBin -Force

        $installedVersion = & $destBin --version 2>&1 |
            Select-String '\d+\.\d+\.\d+' |
            ForEach-Object { $_.Matches[0].Value } |
            Select-Object -First 1
        Write-Ok "Installed: agentverse $installedVersion"

        # Update PATH
        $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($currentPath -notlike "*$InstallDir*") {
            [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$currentPath", "User")
            $env:PATH = "$InstallDir;$env:PATH"
            Write-Ok "Added to user PATH"
        }

        Write-Host ""
        Write-Ok "agentverse installed successfully!"
        Write-Host ""
        Write-Host "  Run: agentverse --help" -ForegroundColor Gray
        Write-Host "  Self-update: agentverse self-update" -ForegroundColor Gray
        Write-Host "  Docs: https://github.com/$RepoOwner/$RepoName" -ForegroundColor Gray
        Write-Host ""
        Write-Host "  Restart your terminal or run:" -ForegroundColor Gray
        Write-Host "    `$env:PATH = `"$InstallDir;`$env:PATH`"" -ForegroundColor Gray
    }
    finally {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Main

