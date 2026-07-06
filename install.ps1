<#
.SYNOPSIS
  8sync standalone installer for Windows (PowerShell).

.DESCRIPTION
  Downloads the prebuilt `8sync.exe` binary from GitHub Releases — no git clone,
  no Rust toolchain, no cargo build. Ideal for a fresh machine or quick upgrade.

  One-liner:
    irm https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/install.ps1 | iex

  Upgrade:   re-run the same command (atomically replaces the old binary).
  Uninstall: download the script and run:  .\install.ps1 -Uninstall

.PARAMETER Uninstall
  Remove the installed 8sync.exe and exit.

.NOTES
  Environment:
    SUSYNC_VERSION   release tag to install (default: latest, e.g. v0.12.1)
    SUSYNC_BIN_DIR   install location (default: %LOCALAPPDATA%\Programs\8sync)
#>
param(
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

$Repo = '8-Sync-Dev/su-code'
$BinDir = if ($env:SUSYNC_BIN_DIR) { $env:SUSYNC_BIN_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\8sync' }
$BinName = '8sync.exe'
$BinPath = Join-Path $BinDir $BinName

# --- helpers ---------------------------------------------------------------

# Pull the Location header off a response object (or an exception's .Response),
# coping with the different shapes across Windows PowerShell 5.1 and PS 7+.
function Get-HeaderLocation($obj) {
    if ($null -eq $obj) { return $null }
    try {
        $h = $obj.Headers
        if ($h) {
            # Typed property (HttpResponseHeaders / WebHeaderCollection).
            try { if ($h.Location) { return $h.Location.ToString() } } catch {}
            # Dictionary-style indexer (BasicHtmlWebResponseObject).
            try {
                $v = $h['Location']
                if ($v) { return ($v | Select-Object -First 1).ToString() }
            } catch {}
        }
    } catch {}
    return $null
}

# Resolve the latest release tag.
#
# Prefer the releases/latest *web* redirect over the GitHub API: the
# unauthenticated API is rate-limited to 60 req/hour per IP (403 once
# exhausted). The redirect (github.com/<repo>/releases/latest ->
# .../releases/tag/vX.Y.Z) is not.
function Get-LatestVersion {
    $latestUrl = "https://github.com/$Repo/releases/latest"
    $loc = $null
    try {
        # -MaximumRedirection 0: PS7 returns the 3xx response; 5.1 throws (caught below).
        $resp = Invoke-WebRequest -Uri $latestUrl -MaximumRedirection 0 -UseBasicParsing -ErrorAction Stop
        $loc = Get-HeaderLocation $resp
    } catch {
        $loc = Get-HeaderLocation $_.Exception.Response
    }
    if ($loc -and $loc -match '/releases/tag/([^/?#]+)') {
        return $matches[1].Trim()
    }
    # Fallback: GitHub API (needs a User-Agent or it 403s).
    try {
        $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
        $tag = (Invoke-RestMethod -Uri $apiUrl -UseBasicParsing -Headers @{ 'User-Agent' = '8sync-installer' }).tag_name
        if ($tag) { return $tag }
    } catch {}
    return $null
}

# --- uninstall -------------------------------------------------------------

if ($Uninstall) {
    if (Test-Path -LiteralPath $BinPath) {
        Remove-Item -LiteralPath $BinPath -Force
        Write-Host "8sync uninstalled (removed $BinPath)."
    } else {
        Write-Host "8sync not found at $BinPath; nothing to uninstall."
    }
    return
}

# --- resolve version -------------------------------------------------------

# Ensure TLS 1.2 on older Windows PowerShell (5.1 defaults can be too weak).
try {
    [Net.ServicePointManager]::SecurityProtocol = `
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {}

$version = $env:SUSYNC_VERSION
if (-not $version) {
    $version = Get-LatestVersion
}
if (-not $version) {
    throw "8sync: could not resolve latest version; set `$env:SUSYNC_VERSION (e.g. `$env:SUSYNC_VERSION='v0.12.1')."
}
# Release tags are vX.Y.Z; accept a bare X.Y.Z in SUSYNC_VERSION too.
if ($version -notlike 'v*') { $version = "v$version" }

# --- download + install ----------------------------------------------------

$asset = "8sync-$version-windows-x86_64.exe"
$url = "https://github.com/$Repo/releases/download/$version/$asset"
Write-Host "Installing 8sync $version (windows-x86_64)..."

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("8sync-" + [System.Guid]::NewGuid().ToString('N') + ".exe")
try {
    Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing
} catch {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
    throw "8sync: download failed: $url`n$($_.Exception.Message)"
}
if (-not (Test-Path -LiteralPath $tmp) -or (Get-Item -LiteralPath $tmp).Length -eq 0) {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
    throw "8sync: downloaded an empty file from $url"
}

# Atomically replace any existing binary (upgrade path).
Move-Item -LiteralPath $tmp -Destination $BinPath -Force

Write-Host "Installed -> $BinPath"
try { & $BinPath --version } catch {}

# --- PATH ------------------------------------------------------------------

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$segments = @()
if ($userPath) { $segments = $userPath -split ';' | Where-Object { $_ -ne '' } }
$already = $segments | Where-Object { $_.TrimEnd('\') -ieq $BinDir.TrimEnd('\') }
if (-not $already) {
    $newPath = if ($userPath) { "$userPath;$BinDir" } else { $BinDir }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    # Reflect it in the current session too.
    $env:Path = "$env:Path;$BinDir"
    Write-Host ""
    Write-Host "$BinDir added to your user PATH."
    Write-Host "Restart your shell (or open a new terminal) for the change to take effect."
}

# --- next steps ------------------------------------------------------------

Write-Host ""
Write-Host "Done. Next steps:"
Write-Host "  8sync setup        # full stack + config"
Write-Host "  8sync doctor       # verify"
Write-Host "  8sync up           # upgrade later (or re-run this installer)"
