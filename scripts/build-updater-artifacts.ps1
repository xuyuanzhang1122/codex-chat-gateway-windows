param(
    [string]$OutputDirectory = '',
    [string]$ReleaseBaseUrl = '',
    [string]$Notes = ''
)

# Sign the full Studio (Inno) installer and build latest.json for GitHub Releases.
# Requires TAURI_SIGNING_PRIVATE_KEY or TAURI_SIGNING_PRIVATE_KEY_PATH.
# Private keys must NEVER be committed to the repository.

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

function Get-Sha256Hash {
    param([Parameter(Mandatory = $true)][string]$Path)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        try {
            return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
        }
        finally {
            $stream.Dispose()
        }
    }
    finally {
        $sha256.Dispose()
    }
}

if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $projectRoot 'dist-installer'
}
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$projectPath = [IO.Path]::GetFullPath($projectRoot).TrimEnd('\')
$projectPrefix = $projectPath + '\'
if (-not $outputRoot.StartsWith($projectPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'OutputDirectory must be inside the project directory.'
}

$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
    throw "Invalid VERSION: $version"
}

# Keep tauri.conf.json / Cargo.toml in lockstep with the root VERSION.
& (Join-Path $PSScriptRoot 'sync-versions.ps1') -ProjectRoot $projectRoot

if (-not $ReleaseBaseUrl) {
    $ReleaseBaseUrl = "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/download/v$version"
}

# Resolve signing private key (never read from repo paths that might be committed).
if (-not $env:TAURI_SIGNING_PRIVATE_KEY -and -not $env:TAURI_SIGNING_PRIVATE_KEY_PATH) {
    $defaultKey = Join-Path $env:USERPROFILE '.codex-chat-gateway\tauri-updater.key'
    if (Test-Path -LiteralPath $defaultKey) {
        $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $defaultKey
        Write-Host "Using private key: $defaultKey" -ForegroundColor DarkGray
    }
    else {
        throw @"
Missing updater signing key.
Set one of:
  `$env:TAURI_SIGNING_PRIVATE_KEY = '<key contents>'
  `$env:TAURI_SIGNING_PRIVATE_KEY_PATH = 'C:\path\to\tauri-updater.key'
Or generate:
  cd desktop-tauri
  npx tauri signer generate -w `$env:USERPROFILE\.codex-chat-gateway\tauri-updater.key --ci
"@
    }
}

$tauriDir = Join-Path $projectRoot 'desktop-tauri'
if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'package.json'))) {
    throw 'desktop-tauri project is missing.'
}

Write-Host ''
Write-Host '=== Codex Chat Gateway updater artifacts ===' -ForegroundColor Cyan
Write-Host "Version: $version"
Write-Host "Channel: HTTPS GitHub Releases"
Write-Host ''

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

$installerName = "CodexChatGateway-Studio-Setup-v$version.exe"
$installerPath = Join-Path $outputRoot $installerName
if (-not (Test-Path -LiteralPath $installerPath)) {
    throw "Full Studio installer is missing: $installerPath. Run build-tauri-installer.ps1 first."
}

if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'node_modules'))) {
    Push-Location $tauriDir
    try {
        Write-Host 'npm install...'
        npm install
        if ($LASTEXITCODE -ne 0) { throw 'npm install failed.' }
    }
    finally {
        Pop-Location
    }
}

$hasKeyText = -not [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY)
$hasKeyPath = -not [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY_PATH)
if ($hasKeyText -and $hasKeyPath) {
    throw 'Set only one of TAURI_SIGNING_PRIVATE_KEY or TAURI_SIGNING_PRIVATE_KEY_PATH.'
}
$signingKeyPath = ''
if ($hasKeyPath) {
    $signingKeyPath = $env:TAURI_SIGNING_PRIVATE_KEY_PATH
    $env:TAURI_SIGNING_PRIVATE_KEY = $null
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $null
}
else {
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $null
}
$hasKeyPassword = -not [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD)
if (-not $hasKeyPassword) {
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $null
}

$signaturePath = $installerPath + '.sig'
if (Test-Path -LiteralPath $signaturePath) {
    Remove-Item -LiteralPath $signaturePath -Force
}

Push-Location $tauriDir
try {
    Write-Host 'Signing the full Studio (Inno) installer...'
    if ($signingKeyPath) {
        if ($hasKeyPassword) {
            npx tauri signer sign --private-key-path $signingKeyPath $installerPath
        }
        else {
            # Keep the empty value attached: some PowerShell native argument
            # modes drop a separate empty string and shift FILE into PASSWORD.
            npx tauri signer sign --private-key-path $signingKeyPath --password= $installerPath
        }
    }
    else {
        if ($hasKeyPassword) {
            npx tauri signer sign $installerPath
        }
        else {
            npx tauri signer sign --password= $installerPath
        }
    }
    if ($LASTEXITCODE -ne 0) { throw 'Signing the full Studio installer failed.' }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $signaturePath)) {
    throw "Signature file was not created: $signaturePath"
}

$signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($signature)) {
    throw 'Signature file is empty.'
}

$pubDate = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
if (-not $Notes) {
    $Notes = "Codex Chat Gateway Studio $version"
}

$downloadUrl = "$ReleaseBaseUrl/$installerName"
$latest = [ordered]@{
    version  = $version
    notes    = $Notes
    pub_date = $pubDate
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = $signature
            url       = $downloadUrl
        }
    }
}

$latestPath = Join-Path $outputRoot 'latest.json'
$latest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $latestPath -Encoding utf8

$shaInstaller = Get-Sha256Hash -Path $installerPath
Set-Content -LiteralPath ($installerPath + '.sha256') -Value "$shaInstaller  $installerName" -Encoding ascii

# Remove obsolete bare-console updater outputs from local build directories.
Get-ChildItem -LiteralPath $outputRoot -Filter "CodexChatGateway-Studio-Updater-v$version*" -File -ErrorAction SilentlyContinue |
    Remove-Item -Force

Write-Host ''
Write-Host 'Updater artifacts ready:' -ForegroundColor Green
Write-Host "  $installerPath"
Write-Host "  $signaturePath"
Write-Host "  $latestPath"
Write-Host ''
Write-Host 'Upload to GitHub Release (same tag as VERSION):'
Write-Host "  $installerName"
Write-Host "  $installerName.sig"
Write-Host '  latest.json'
Write-Host ''
Write-Host 'Endpoint used by the app:'
Write-Host '  https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest/download/latest.json'
Write-Host ''
Write-Host 'Note: .gateway/models.json is never modified by the updater.' -ForegroundColor DarkGray
