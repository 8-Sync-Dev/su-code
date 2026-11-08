<#
.SYNOPSIS
  8sync standalone installer for Windows (PowerShell).

.DESCRIPTION
  Downloads the prebuilt `8sync.exe` binary from the GitHub Releases of the
  PRIVATE repo 8syncdev/kilop — no git clone, no Rust toolchain, no cargo build.
  The repo is private, so a GitHub token with read access to it is required:
  $env:GH_TOKEN, else $env:GITHUB_TOKEN, else `gh auth token` (run `gh auth login` first).

  One-liner:
    gh api repos/8syncdev/kilop/contents/install.ps1 -H "Accept: application/vnd.github.raw" | iex

  After the binary lands, the installer runs `8sync setup --no-profile`: the AI
  harness (gh, omp, forge + zsh, configs, skills, `harness global`).

  Upgrade:   re-run the same command (atomically replaces the old binary), or `8sync up`.
  Uninstall: save the script and run:  .\install.ps1 -Uninstall

.PARAMETER Uninstall
  Remove the installed 8sync.exe and exit.

.NOTES
  Environment:
    GH_TOKEN / GITHUB_TOKEN  GitHub token with read access to 8syncdev/kilop
    SUSYNC_VERSION   release tag to install (default: latest, e.g. v0.141.0)
    SUSYNC_BIN_DIR   install location (default: %LOCALAPPDATA%\Programs\8sync)
    SUSYNC_NO_SETUP  set to 1 to install the binary only (skip `8sync setup`)
#>
param(
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

$Repo = '8syncdev/kilop'
$BinDir = if ($env:SUSYNC_BIN_DIR) { $env:SUSYNC_BIN_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\8sync' }
$BinName = '8sync.exe'
$BinPath = Join-Path $BinDir $BinName

# --- helpers ---------------------------------------------------------------

# The machine's GitHub token, in the order `8sync up` uses (crates/cli/src/verbs/selfup.rs
# github_token): GH_TOKEN, GITHUB_TOKEN, then `gh auth token`. Anything that is not one plain
# token is ignored.
function Get-GitHubToken {
    $candidates = @($env:GH_TOKEN, $env:GITHUB_TOKEN)
    foreach ($t in $candidates) {
        if ($t -and "$t".Trim() -match '^[A-Za-z0-9_.-]+$') { return "$t".Trim() }
    }
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        try {
            $t = (& gh auth token 2>$null | Out-String).Trim()
            if ($LASTEXITCODE -eq 0 -and $t -match '^[A-Za-z0-9_.-]+$') { return $t }
        } catch {}
    }
    return $null
}

function Get-ApiHeaders($accept) {
    @{
        'Authorization'        = "Bearer $Token"
        'Accept'               = $accept
        'X-GitHub-Api-Version' = '2022-11-28'
        'User-Agent'           = '8sync-install'
    }
}

# Detect the machine architecture and map it onto the release asset labels
# (scripts/ci/publish-release.sh names assets 8sync-<tag>-<os>-<arch>[.exe]).
#
# PROCESSOR_ARCHITEW6432 is set only inside a 32-bit process on a 64-bit OS,
# where PROCESSOR_ARCHITECTURE describes the *process* ('x86') rather than the
# machine -- precisely the case that would otherwise mis-target an ARM64 box.
function Get-TargetArch {
    $raw = $env:PROCESSOR_ARCHITEW6432
    if (-not $raw) { $raw = $env:PROCESSOR_ARCHITECTURE }
    if (-not $raw) {
        try { $raw = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() } catch {}
    }
    if (-not $raw) { return 'unknown' }
    switch -Regex ($raw) {
        '^(AMD64|x64)$' { return 'x86_64' }
        '^ARM64$'       { return 'aarch64' }
        default         { return "$raw".ToLowerInvariant() }
    }
}

# Download release asset `$a` (an entry of the release's `assets`) to `$dest` through the API.
# The asset endpoint answers with a redirect to a signed storage URL; Invoke-WebRequest drops the
# Authorization header on a redirect to another host (PS 7 unless -PreserveAuthorizationOnRedirect;
# Windows PowerShell 5.1's HttpWebRequest clears it on auto-redirect), so the token never reaches
# the storage host.
function Save-Asset($a, $dest) {
    Invoke-WebRequest -Uri "https://api.github.com/repos/$Repo/releases/assets/$($a.id)" `
        -Headers (Get-ApiHeaders 'application/octet-stream') -OutFile $dest -UseBasicParsing
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

# --- token + release -------------------------------------------------------

# Ensure TLS 1.2 on older Windows PowerShell (5.1 defaults can be too weak).
try {
    [Net.ServicePointManager]::SecurityProtocol = `
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {}

$Token = Get-GitHubToken
if (-not $Token) {
    throw "8sync: $Repo is private - set `$env:GH_TOKEN (or `$env:GITHUB_TOKEN), or run ``gh auth login``, with read access to it, then re-run."
}

$version = $env:SUSYNC_VERSION
if ($version -and $version -notlike 'v*') { $version = "v$version" }   # accept a bare X.Y.Z
$relPath = if ($version) { "tags/$version" } else { 'latest' }
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/$relPath" `
        -Headers (Get-ApiHeaders 'application/vnd.github+json') -UseBasicParsing
} catch {
    throw "8sync: could not read release '$relPath' of $Repo - check the token can read the repo (gh auth status) and the tag exists.`n$($_.Exception.Message)"
}
$version = "$($release.tag_name)"
if ($version -notmatch '^v[0-9]') {
    throw "8sync: release '$relPath' of $Repo has no version tag."
}

# --- download + install ----------------------------------------------------

# An ARM64 machine must not silently receive the x64 build: releases ship Windows x86_64 only,
# so name the missing target instead of installing the wrong binary.
$arch = Get-TargetArch
$target = "windows-$arch"
$asset = "8sync-$version-$target.exe"
$entry = $release.assets | Where-Object { $_.name -eq $asset } | Select-Object -First 1
if (-not $entry) {
    throw "8sync: no prebuilt binary for $target in release $version (missing asset: $asset).`nBuild from source instead: https://github.com/$Repo (cargo build --release)."
}

Write-Host "Installing 8sync $version ($target)..."

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# The temp file is a *sibling* of the destination, not %TEMP%: a move across
# volumes is a copy rather than a rename, which would make a half-written
# binary observable at $BinPath. Mirrors download_and_replace() in
# crates/cli/src/verbs/selfup.rs.
# The leading dot matches selfup.rs's naming, but it makes the file "hidden"
# to the PowerShell provider on Unix-hosted pwsh, so every probe of $tmp needs
# -Force. (On Windows a leading dot sets no hidden attribute; -Force is a
# harmless no-op there.)
$tmp = Join-Path $BinDir (".8sync.new." + $PID)
try {
    Save-Asset $entry $tmp
} catch {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
    throw "8sync: download failed: $asset ($Repo $version)`n$($_.Exception.Message)"
}
if (-not (Test-Path -LiteralPath $tmp) -or (Get-Item -LiteralPath $tmp -Force).Length -eq 0) {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
    throw "8sync: downloaded an empty file for $asset"
}

# Integrity: the asset's API `digest` ("sha256:<hex>"), else the release's `<asset>.sha256` file
# (written by scripts/ci/publish-release.sh). A mismatch is fatal; no digest anywhere is a loud
# notice, the same policy as `8sync up`.
$wantHash = $null
if ("$($entry.digest)" -match '^sha256:([0-9a-fA-F]{64})$') {
    $wantHash = $matches[1]
} else {
    $sideEntry = $release.assets | Where-Object { $_.name -eq "$asset.sha256" } | Select-Object -First 1
    if ($sideEntry) {
        $side = Join-Path $BinDir (".8sync.sha256." + $PID)
        try {
            Save-Asset $sideEntry $side
            $line = (Get-Content -LiteralPath $side -TotalCount 1 -Force)
            if ("$line" -match '^([0-9a-fA-F]{64})\b') { $wantHash = $matches[1] }
        } catch {
        } finally {
            Remove-Item -LiteralPath $side -Force -ErrorAction SilentlyContinue
        }
    }
}
# .NET, not Get-FileHash: Windows PowerShell started from pwsh 7 inherits a
# PSModulePath that loads pwsh's Microsoft.PowerShell.Utility, which hides
# Get-FileHash — and the checksum then silently never ran.
if ($wantHash) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($tmp)
    try { $gotHash = -join ($sha.ComputeHash($stream) | ForEach-Object { $_.ToString('X2') }) }
    finally { $stream.Dispose(); $sha.Dispose() }
    if ($gotHash -ne $wantHash) {
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
        throw "8sync: checksum mismatch for $asset - refusing to install.`n  expected sha256:$($wantHash.ToLowerInvariant())`n  actual   sha256:$($gotHash.ToLowerInvariant())"
    }
    Write-Host "  checksum: ok (sha256:$($wantHash.ToLowerInvariant()))"
} else {
    Write-Host "  checksum: NOT VERIFIED (no sha256 digest published for $asset)"
}

# Windows cannot overwrite a running .exe (an open `8sync . <session>` holds it),
# but CAN rename it aside: move the live binary to .8sync.old.<pid>, drop the new
# one in, restore on failure. Mirrors selfup.rs, which also sweeps the leftovers.
Get-ChildItem -LiteralPath $BinDir -Filter '.8sync.old.*' -Force -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue
$old = $null
if (Test-Path -LiteralPath $BinPath) {
    $old = Join-Path $BinDir (".8sync.old." + $PID)
    Move-Item -LiteralPath $BinPath -Destination $old -Force
}
try {
    Move-Item -LiteralPath $tmp -Destination $BinPath -Force
} catch {
    if ($old) { Move-Item -LiteralPath $old -Destination $BinPath -Force -ErrorAction SilentlyContinue }
    Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    throw "8sync: could not install $BinPath`n$($_.Exception.Message)"
}
if ($old) { Remove-Item -LiteralPath $old -Force -ErrorAction SilentlyContinue }

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

# --- harness setup -----------------------------------------------------------

Write-Host ""
if ($env:SUSYNC_NO_SETUP -eq '1') {
    Write-Host "SUSYNC_NO_SETUP=1 - skipped setup. Run it any time: 8sync setup --no-profile"
} else {
    Write-Host "Running 8sync setup --no-profile (AI harness: omp, forge, skills, MCP)..."
    # Native-command failures must not abort the installer under 'Stop'; setup
    # reports its own per-step summary.
    $ErrorActionPreference = 'Continue'
    & $BinPath setup --no-profile
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Setup finished - the summary above lists any step to retry."
    } else {
        Write-Warning "8sync setup exited with code $LASTEXITCODE - re-run: 8sync setup --no-profile"
    }
    # Installers run by setup (omp, forge) extend the user PATH; pick
    # those up so this window can use them immediately.
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'User') + ';' + [Environment]::GetEnvironmentVariable('Path', 'Machine')
}

# --- next steps ------------------------------------------------------------

Write-Host ""
Write-Host "Next steps:"
Write-Host "  8sync doctor             # verify"
Write-Host "  cd <project>; 8sync .    # omp session   |   8sync forge <name>   # Forge zsh session"
Write-Host "  8sync up                 # upgrade later (8sync also notifies when a release is out)"
