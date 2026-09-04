<#
.SYNOPSIS
  Generate an AI image from a text prompt via gpt-image-2 (OpenAI / api.apikey.fun).

.DESCRIPTION
  Calls https://api.apikey.fun/v1/images/generations using gpt-image-2,
  decodes base64 PNG data, and saves to the specified output file.

.EXAMPLE
  .\scripts\gen-img.ps1 "a high tech neon robotic 8 logo" -Output "logo.png"
  .\scripts\gen-img.ps1 "architecture diagram of modern microservices" -Size "1024x1024"
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$Prompt,

    [Parameter(Position=1)]
    [string]$Output = "./generated-image.png",

    [string]$Model = "gpt-image-2",

    [string]$Size = "1024x1024",

    [string]$ApiKey = $env:OPENAI_API_KEY,

    [string]$Endpoint = "https://api.apikey.fun/v1/images/generations"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    # Try reading from ~/.omp/agent/models.yml
    $modelsYml = Join-Path $env:USERPROFILE ".omp\agent\models.yml"
    if (Test-Path $modelsYml) {
        $lines = Get-Content $modelsYml
        $inProvider = $false
        foreach ($line in $lines) {
            $trimmed = $line.Trim()
            if ($trimmed -eq "codex:" -or $trimmed -eq "openai:") {
                $inProvider = $true
                continue
            }
            if ($inProvider -and ($line.StartsWith("  ") -or $line.StartsWith("    "))) {
                if ($trimmed.StartsWith("apiKey:")) {
                    $ApiKey = $trimmed.Substring(7).Trim().Trim('"').Trim("'")
                    break
                }
            } elseif (-not $line.StartsWith("  ") -and -not $line.StartsWith("    ") -and -not [string]::IsNullOrWhiteSpace($line)) {
                $inProvider = $false
            }
        }
    }
}

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    Write-Error "No API key found. Set `$env:OPENAI_API_KEY or pass -ApiKey."
    exit 1
}

Write-Host "-> Generating image with $Model (size: $Size)..." -ForegroundColor Cyan
Write-Host "   Prompt: `"$Prompt`"" -ForegroundColor Gray

$body = @{
    model  = $Model
    prompt = $Prompt
    size   = $Size
    n      = 1
} | ConvertTo-Json -Compress

$headers = @{
    "Authorization"               = "Bearer $ApiKey"
    "x-openai-actor-authorization" = "apikey.fun"
    "Content-Type"                = "application/json"
}

try {
    $resp = Invoke-RestMethod -Uri $Endpoint -Method Post -Headers $headers -Body $body -TimeoutSec 120
} catch {
    Write-Error "API request failed: $_"
    exit 1
}

$outDir = Split-Path $Output -Parent
if (-not [string]::IsNullOrWhiteSpace($outDir) -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

if ($resp.data -and $resp.data[0].b64_json) {
    $b64 = $resp.data[0].b64_json
    $bytes = [Convert]::FromBase64String($b64)
    [System.IO.File]::WriteAllBytes($Output, $bytes)
    $kb = [math]::Round($bytes.Length / 1024, 1)
    Write-Host "v Saved image ($kb KB) to: $Output" -ForegroundColor Green
} elseif ($resp.data -and $resp.data[0].url) {
    $url = $resp.data[0].url
    Write-Host "-> Downloading from $url..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $url -OutFile $Output
    $size = (Get-Item $Output).Length
    $kb = [math]::Round($size / 1024, 1)
    Write-Host "v Downloaded image ($kb KB) to: $Output" -ForegroundColor Green
} else {
    Write-Error "Unrecognized response format: $($resp | ConvertTo-Json -Depth 3)"
    exit 1
}

Write-Host "Tip: VLM agents can view this image directly with tool read path: `"$Output`"" -ForegroundColor DarkGray
