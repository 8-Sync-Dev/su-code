<#
.SYNOPSIS
    Auto-setup Claude Code settings & omp Anthropic models from JSON.
.DESCRIPTION
    Takes the Claude Code JSON configuration and automatically:
    1. Writes C:\Users\<User>\.claude\settings.json
    2. Configures Windows User environment variables (ANTHROPIC_BASE_URL, ANTHROPIC_AUTH_TOKEN, etc.)
    3. Injects all 9 modern Claude models into ~/.omp/agent/models.yml
    4. Sets default modelRole in ~/.omp/agent/config.yml to anthropic/claude-fable-5-1:high
    5. Runs 8sync harness claude-code if 8sync binary is present.
.EXAMPLE
    .\scripts\setup-claude-key.ps1 '{"env":{"ANTHROPIC_BASE_URL":"https://api.apikey.fun","ANTHROPIC_AUTH_TOKEN":"your-api-key"}}'
#>
[CmdletBinding()]
param(
    [Parameter(Position=0, ValueFromPipeline=$true)]
    [string]$ConfigJson
)

$ErrorActionPreference = "Stop"

if (-not $ConfigJson) {
    if ($input) {
        $ConfigJson = $input | Out-String
    }
}

if (-not $ConfigJson) {
    Write-Host "Please paste your Claude Code settings JSON (end with Enter):" -ForegroundColor Cyan
    $ConfigJson = Read-Host
}

# If it's a file path
if (Test-Path $ConfigJson -PathType Leaf) {
    $ConfigJson = Get-Content -Raw -Path $ConfigJson
}

try {
    $parsed = $ConfigJson | ConvertFrom-Json
} catch {
    Write-Error "Invalid JSON input: $_"
    exit 1
}

$baseUrl = $null
$token = $null
$traffic = "1"
$attrHeader = "0"

if ($parsed.env) {
    $baseUrl = $parsed.env.ANTHROPIC_BASE_URL
    $token = $parsed.env.ANTHROPIC_AUTH_TOKEN
    if ($parsed.env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC) {
        $traffic = [string]$parsed.env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC
    }
    if ($parsed.env.CLAUDE_CODE_ATTRIBUTION_HEADER) {
        $attrHeader = [string]$parsed.env.CLAUDE_CODE_ATTRIBUTION_HEADER
    }
} else {
    $baseUrl = $parsed.ANTHROPIC_BASE_URL
    $token = $parsed.ANTHROPIC_AUTH_TOKEN
}

if (-not $baseUrl -or -not $token) {
    Write-Error "Missing ANTHROPIC_BASE_URL or ANTHROPIC_AUTH_TOKEN in JSON."
    exit 1
}

$baseUrl = $baseUrl.TrimEnd('/')

Write-Host "==> Endpoint: $baseUrl" -ForegroundColor Green
Write-Host "==> Token:    $($token.Substring(0, [Math]::Min(12, $token.Length)))..." -ForegroundColor Green

# 1. Write ~/.claude/settings.json
$claudeDir = Join-Path $HOME ".claude"
if (-not (Test-Path $claudeDir)) {
    New-Item -ItemType Directory -Path $claudeDir -Force | Out-Null
}
$settingsPath = Join-Path $claudeDir "settings.json"
$settingsObj = [ordered]@{
    '$schema' = "https://json.schemastore.org/claude-code-settings.json"
    'env' = [ordered]@{
        'ANTHROPIC_BASE_URL' = $baseUrl
        'ANTHROPIC_AUTH_TOKEN' = $token
        'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC' = $traffic
        'CLAUDE_CODE_ATTRIBUTION_HEADER' = $attrHeader
    }
}
$settingsObj | ConvertTo-Json -Depth 5 | Set-Content -Path $settingsPath -Encoding UTF8
Write-Host "✓ Wrote $settingsPath" -ForegroundColor Green

# 2. Persist User environment variables
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_BASE_URL', $baseUrl, [System.EnvironmentVariableTarget]::User)
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_AUTH_TOKEN', $token, [System.EnvironmentVariableTarget]::User)
[System.Environment]::SetEnvironmentVariable('CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC', $traffic, [System.EnvironmentVariableTarget]::User)
[System.Environment]::SetEnvironmentVariable('CLAUDE_CODE_ATTRIBUTION_HEADER', $attrHeader, [System.EnvironmentVariableTarget]::User)
Write-Host "✓ Persisted Windows User Environment Variables" -ForegroundColor Green

# 3. Call 8sync harness if available
if (Get-Command 8sync -ErrorAction SilentlyContinue) {
    Write-Host "==> Executing 8sync harness claude-code..." -ForegroundColor Cyan
    & 8sync harness claude-code $ConfigJson
} else {
    Write-Host "Note: 8sync binary not found in PATH; settings.json and environment variables are active." -ForegroundColor Yellow
}
