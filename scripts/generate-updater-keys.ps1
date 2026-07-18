param(
    [string]$KeyPath = '',
    [switch]$Force
)

# Generate Tauri minisign keypair for Studio auto-update.
# Public key goes into desktop-tauri/src-tauri/tauri.conf.json (safe to commit).
# Private key stays OUTSIDE the repo (or as a CI secret only).

$ErrorActionPreference = 'Stop'

if (-not $KeyPath) {
    $dir = Join-Path $env:USERPROFILE '.codex-chat-gateway'
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $KeyPath = Join-Path $dir 'tauri-updater.key'
}

$tauriDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'desktop-tauri'
if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'package.json'))) {
    throw 'desktop-tauri project is missing.'
}

if ((Test-Path -LiteralPath $KeyPath) -and -not $Force) {
    throw "Key already exists: $KeyPath (pass -Force to overwrite)"
}

Push-Location $tauriDir
try {
    $env:CI = '1'
    if ($Force) {
        npx tauri signer generate -w $KeyPath -f --ci
    }
    else {
        npx tauri signer generate -w $KeyPath --ci
    }
    if ($LASTEXITCODE -ne 0) { throw 'tauri signer generate failed.' }
}
finally {
    Pop-Location
}

$pubPath = "$KeyPath.pub"
if (-not (Test-Path -LiteralPath $pubPath)) {
    throw "Public key file missing: $pubPath"
}

$pubkey = (Get-Content -LiteralPath $pubPath -Raw).Trim()
Write-Host ''
Write-Host '=== Updater keypair ready ===' -ForegroundColor Green
Write-Host "Private key (SECRET): $KeyPath"
Write-Host "Public key file:      $pubPath"
Write-Host ''
Write-Host 'Put this public key into desktop-tauri/src-tauri/tauri.conf.json -> plugins.updater.pubkey:'
Write-Host $pubkey
Write-Host ''
Write-Host 'CI / local signing env:'
Write-Host ('  $env:TAURI_SIGNING_PRIVATE_KEY_PATH = "{0}"' -f $KeyPath)
Write-Host '  # or paste file contents into TAURI_SIGNING_PRIVATE_KEY secret'
Write-Host ''
Write-Host 'NEVER commit the private key.' -ForegroundColor Yellow
