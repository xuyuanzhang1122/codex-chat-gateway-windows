param(
    [string]$OutputDirectory = '',
    [string]$ReleaseBaseUrl = '',
    [string]$Notes = '',
    [switch]$SkipBuild
)

# Build signed Tauri updater artifacts + latest.json for GitHub Releases.
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
$projectPath = [IO.Path]::GetFullPath($projectRoot)
if (-not $outputRoot.StartsWith($projectPath, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'OutputDirectory must be inside the project directory.'
}

$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Invalid VERSION: $version"
}

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

if (-not $SkipBuild) {
    Push-Location $tauriDir
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'node_modules'))) {
            Write-Host 'npm install...'
            npm install
            if ($LASTEXITCODE -ne 0) { throw 'npm install failed.' }
        }
        Write-Host 'tauri build (NSIS + updater artifacts)...'
        # Bundle NSIS so createUpdaterArtifacts emits .nsis.zip + .sig
        npm run tauri build
        if ($LASTEXITCODE -ne 0) { throw 'tauri build failed.' }
    }
    finally {
        Pop-Location
    }
}

$bundleRoot = Join-Path $tauriDir 'src-tauri\target\release\bundle'
if (-not (Test-Path -LiteralPath $bundleRoot)) {
    throw "Bundle directory not found: $bundleRoot"
}

# Tauri 2 on Windows may emit either:
#   - *.nsis.zip + .sig  (zipped updater package)
#   - *-setup.exe + .sig (NSIS installer used as updater payload)
$allFiles = @(
    Get-ChildItem -LiteralPath $bundleRoot -Recurse -File -ErrorAction SilentlyContinue
)
$sigCandidates = @($allFiles | Where-Object { $_.Extension -eq '.sig' })

$payload = $null
# 1) Prefer signed NSIS setup.exe (common Tauri 2 output)
$setupExes = @(
    $allFiles | Where-Object {
        $_.Extension -eq '.exe' -and
        ($_.Name -match 'setup' -or $_.DirectoryName -match 'nsis') -and
        ($_.Name -notmatch 'uninstall')
    }
)
foreach ($exe in ($setupExes | Sort-Object Length -Descending)) {
    $sibling = $exe.FullName + '.sig'
    if (Test-Path -LiteralPath $sibling) {
        $payload = $exe
        break
    }
}

# 2) Fallback: nsis.zip
if (-not $payload) {
    $zips = @(
        $allFiles | Where-Object {
            $_.Name -match '\.nsis\.zip$' -or
            ($_.Extension -eq '.zip' -and $_.Name -match 'nsis|setup|x64')
        }
    )
    foreach ($z in ($zips | Sort-Object Length -Descending)) {
        $sibling = $z.FullName + '.sig'
        if (Test-Path -LiteralPath $sibling) {
            $payload = $z
            break
        }
    }
}

if (-not $payload) {
    throw @"
No signed updater payload found under $bundleRoot
Looked for *-setup.exe.sig and *.nsis.zip.sig.
Ensure createUpdaterArtifacts=true, targets include nsis, and TAURI_SIGNING_PRIVATE_KEY is set.
"@
}

$sigPath = $payload.FullName + '.sig'
if (-not (Test-Path -LiteralPath $sigPath)) {
    $sig = $sigCandidates | Where-Object { $_.Name -eq ($payload.Name + '.sig') } | Select-Object -First 1
    if ($sig) { $sigPath = $sig.FullName }
}
if (-not (Test-Path -LiteralPath $sigPath)) {
    throw "Signature file (.sig) not found for $($payload.FullName)."
}

$ext = $payload.Extension.ToLowerInvariant()
if ($ext -eq '.exe') {
    $outZipName = "CodexChatGateway-Studio-Updater-v$version-windows-x86_64-setup.exe"
} else {
    $outZipName = "CodexChatGateway-Studio-Updater-v$version-windows-x86_64.nsis.zip"
}
$outSigName = "$outZipName.sig"
$outZip = Join-Path $outputRoot $outZipName
$outSig = Join-Path $outputRoot $outSigName
Copy-Item -LiteralPath $payload.FullName -Destination $outZip -Force
Copy-Item -LiteralPath $sigPath -Destination $outSig -Force

$signature = (Get-Content -LiteralPath $outSig -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($signature)) {
    throw 'Signature file is empty.'
}

$pubDate = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
if (-not $Notes) {
    $Notes = "Codex Chat Gateway Studio $version"
}

$downloadUrl = "$ReleaseBaseUrl/$outZipName"
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

$shaZip = Get-Sha256Hash -Path $outZip
Set-Content -LiteralPath ($outZip + '.sha256') -Value "$shaZip  $outZipName" -Encoding ascii

Write-Host ''
Write-Host 'Updater artifacts ready:' -ForegroundColor Green
Write-Host "  $outZip"
Write-Host "  $outSig"
Write-Host "  $latestPath"
Write-Host ''
Write-Host 'Upload to GitHub Release (same tag as VERSION):'
Write-Host "  $outZipName"
Write-Host "  $outSigName"
Write-Host '  latest.json'
Write-Host ''
Write-Host 'Endpoint used by the app:'
Write-Host '  https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest/download/latest.json'
Write-Host ''
Write-Host 'Note: .gateway/models.json is never modified by the updater.' -ForegroundColor DarkGray
