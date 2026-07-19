param(
    [string]$OutputDirectory = '',
    [string]$InnoCompiler = '',
    [switch]$SkipTauriBuild,
    [switch]$SkipRuntimeBootstrap
)

# Builds the STUDIO (Tauri + LobeHub) installer.
# Never packages the legacy C#/WPF UI.

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

# Get-FileHash can be unavailable when PSModulePath points at incompatible
# PowerShell 7 module manifests; compute SHA-256 via .NET instead.
function Get-Sha256Hash {
    param([Parameter(Mandatory = $true)][string]$Path)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        try {
            return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '')
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

# Keep tauri.conf.json / Cargo.toml in lockstep with the root VERSION.
& (Join-Path $PSScriptRoot 'sync-versions.ps1') -ProjectRoot $projectRoot

$tauriDir = Join-Path $projectRoot 'desktop-tauri'
if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'package.json'))) {
    throw 'desktop-tauri project is missing.'
}

Write-Host ''
Write-Host '=== Codex Chat Gateway STUDIO installer ===' -ForegroundColor Cyan
Write-Host "Version: $version"
Write-Host 'UI:      Tauri + LobeHub (NOT legacy C#/WPF)'
Write-Host ''

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
$stage = Join-Path $outputRoot "studio-payload-v$version"
if (Test-Path -LiteralPath $stage) {
    Remove-Item -LiteralPath $stage -Recurse -Force
}
New-Item -ItemType Directory -Path $stage -Force | Out-Null

# 1) Build Tauri release binary
$tauriReleaseDir = Join-Path $tauriDir 'src-tauri\target\release'
$tauriExeName = 'codex-chat-gateway-desktop.exe'
$tauriExe = Join-Path $tauriReleaseDir $tauriExeName

if (-not $SkipTauriBuild) {
    Push-Location $tauriDir
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'node_modules'))) {
            Write-Host 'npm install...'
            npm install
            if ($LASTEXITCODE -ne 0) {
                throw 'npm install failed.'
            }
        }
        Write-Host 'tauri build --no-bundle (Studio console)...'
        npm run tauri build -- --no-bundle
        if ($LASTEXITCODE -ne 0) {
            throw 'tauri build failed.'
        }
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $tauriExe)) {
    $minBytes = 5 * 1024 * 1024
    $alt = Get-ChildItem -LiteralPath $tauriReleaseDir -Filter '*.exe' -File -ErrorAction SilentlyContinue |
        Where-Object {
            ($_.Name -notmatch 'uninstall|nsis|setup') -and ($_.Length -gt $minBytes)
        } |
        Sort-Object Length -Descending |
        Select-Object -First 1
    if ($alt) {
        $tauriExe = $alt.FullName
    }
}

if (-not (Test-Path -LiteralPath $tauriExe)) {
    throw "Tauri Studio executable not found under $tauriReleaseDir"
}

$tauriSize = (Get-Item -LiteralPath $tauriExe).Length
$minBytes = 5 * 1024 * 1024
if ($tauriSize -lt $minBytes) {
    throw "Refusing binary that looks like legacy WPF (size=$tauriSize). Expected Tauri exe > 5MB. Path: $tauriExe"
}

$sizeMb = [math]::Round($tauriSize / 1MB, 1)
Write-Host "Studio console binary: $tauriExe ($sizeMb MB)" -ForegroundColor Green

# 2) Embedded Python runtime (LiteLLM) - NEVER via build-portable (WPF)
$runtimeDir = Join-Path $projectRoot 'runtime'
$runtimeSource = $null
if (Test-Path -LiteralPath (Join-Path $runtimeDir 'python.exe')) {
    $runtimeSource = $runtimeDir
    Write-Host "Using existing project runtime: $runtimeSource"
}
else {
    if ($SkipRuntimeBootstrap) {
        throw 'runtime\python.exe is missing. Remove -SkipRuntimeBootstrap or run build-embedded-runtime.ps1 first.'
    }
    $runtimeSource = Join-Path $outputRoot "studio-runtime-v$version"
    Write-Host 'Building embedded Python runtime (no C#/WPF desktop)...' -ForegroundColor Yellow
    & (Join-Path $PSScriptRoot 'build-embedded-runtime.ps1') -DestinationRuntimeDir $runtimeSource
    if ($LASTEXITCODE -ne 0) {
        throw 'Embedded runtime build failed.'
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $runtimeSource 'python.exe'))) {
    throw "Runtime incomplete: $runtimeSource"
}

# 3) Assemble Studio payload
$rootFiles = @(
    'run_gateway.py',
    'gateway_runtime.py',
    'config.yaml',
    'requirements.txt',
    'VERSION',
    'LICENSE',
    'AGENTS.md',
    'README.md',
    'CHANGELOG.md',
    'THIRD_PARTY_NOTICES.md'
)
foreach ($name in $rootFiles) {
    $src = Join-Path $projectRoot $name
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $stage $name) -Force
    }
}
foreach ($dir in @('scripts', 'patches', 'docs')) {
    $src = Join-Path $projectRoot $dir
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $stage $dir) -Recurse -Force
    }
}

# Main app MUST be Tauri Studio
$stageExe = Join-Path $stage 'CodexChatGateway.exe'
Copy-Item -LiteralPath $tauriExe -Destination $stageExe -Force
Copy-Item -LiteralPath $runtimeSource -Destination (Join-Path $stage 'runtime') -Recurse -Force
Copy-Item -LiteralPath $tauriExe -Destination (Join-Path $stage 'codex-chat-gateway-desktop.exe') -Force

$icon = Join-Path $projectRoot 'desktop\assets\gateway-logo.ico'
if (Test-Path -LiteralPath $icon) {
    Copy-Item -LiteralPath $icon -Destination (Join-Path $stage 'gateway-logo.ico') -Force
}

$launcher = @(
    '@echo off'
    'cd /d "%~dp0"'
    'start "" "%~dp0CodexChatGateway.exe"'
) -join "`r`n"
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $stage 'desktop.bat'), $launcher, $utf8)
[IO.File]::WriteAllText((Join-Path $stage 'Studio.bat'), $launcher, $utf8)
# Chinese launcher name
$cnBat = Join-Path $stage ([char]0x684C + [char]0x9762 + [char]0x7248 + '.bat')
[IO.File]::WriteAllText($cnBat, $launcher, $utf8)

$studioMarker = "Codex Chat Gateway Studio v$version`nUI=Tauri+LobeHub`nhttps://github.com/xuyuanzhang1122/codex-chat-gateway-windows`n"
[IO.File]::WriteAllText((Join-Path $stage 'STUDIO'), $studioMarker, $utf8)

$stageSize = (Get-Item -LiteralPath $stageExe).Length
if ($stageSize -ne $tauriSize) {
    throw "Payload CodexChatGateway.exe size mismatch (stage=$stageSize tauri=$tauriSize). Aborting."
}
if ($stageSize -lt $minBytes) {
    throw "Payload looks like legacy WPF ($stageSize bytes). Aborting."
}

$stageMb = [math]::Round($stageSize / 1MB, 1)
Write-Host "Payload main exe verified: $stageExe ($stageMb MB) = Tauri Studio" -ForegroundColor Green

# 4) Compile Inno Setup (Studio script only)
if (-not $InnoCompiler) {
    $candidates = @(
        (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe')
    )
    $InnoCompiler = $candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
}
if (-not $InnoCompiler -or -not (Test-Path -LiteralPath $InnoCompiler)) {
    throw 'Inno Setup 6 or 7 was not found. Install with: winget install JRSoftware.InnoSetup'
}

$installerScript = Join-Path $projectRoot 'installer\CodexChatGateway-Studio.iss'
if (-not (Test-Path -LiteralPath $installerScript)) {
    throw "Missing $installerScript"
}

$installerPath = Join-Path $outputRoot "CodexChatGateway-Studio-Setup-v$version.exe"
$hashPath = "$installerPath.sha256"
foreach ($path in @($installerPath, $hashPath)) {
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Force
    }
}

Write-Host 'Compiling Inno Studio installer...'
$arguments = @(
    "/DAppVersion=$version",
    "/DPayloadDir=$stage",
    "/DOutputDir=$outputRoot",
    $installerScript
)
& $InnoCompiler $arguments
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $installerPath)) {
    throw 'Studio installer compilation failed.'
}

$hash = Get-Sha256Hash -Path $installerPath
[IO.File]::WriteAllText($hashPath, "$hash  $([IO.Path]::GetFileName($installerPath))`n", $utf8)

Write-Host ''
Write-Host '========================================' -ForegroundColor Green
Write-Host ' STUDIO build complete (Tauri + LobeHub)' -ForegroundColor Green
Write-Host '========================================' -ForegroundColor Green
Write-Host "Payload:   $stage"
Write-Host "Main EXE:  $stageExe  ($stageMb MB)"
Write-Host "Installer: $installerPath"
Write-Host "SHA-256:   $hash"
Write-Host ''
Write-Host 'NOTE: Do NOT use dist-installer\portable* or CodexChatGateway-Setup-v1.2.0.exe' -ForegroundColor Yellow
Write-Host '      Those are the legacy C#/WPF packages.' -ForegroundColor Yellow
Write-Host 'Run the Studio Setup above, or launch:' -ForegroundColor Yellow
Write-Host "  $stageExe" -ForegroundColor Yellow
