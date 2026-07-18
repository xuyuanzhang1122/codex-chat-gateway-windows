param(
    [string]$OutputDirectory = '',
    [string]$InnoCompiler = '',
    [switch]$SkipTauriBuild,
    [switch]$SkipRuntimeBootstrap
)

# Builds the STUDIO (Tauri + LobeHub) installer — never packages the legacy C#/WPF UI.

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $projectRoot 'dist-installer' }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$projectPath = [IO.Path]::GetFullPath($projectRoot)
if (-not $outputRoot.StartsWith($projectPath, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'OutputDirectory must be inside the project directory.'
}

$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Invalid VERSION: $version" }

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

# ── 1) Build Tauri release binary ───────────────────────────────────────────
$tauriReleaseDir = Join-Path $tauriDir 'src-tauri\target\release'
$tauriExeName = 'codex-chat-gateway-desktop.exe'
$tauriExe = Join-Path $tauriReleaseDir $tauriExeName

if (-not $SkipTauriBuild) {
    Push-Location $tauriDir
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'node_modules'))) {
            Write-Host 'npm install…'
            npm install
            if ($LASTEXITCODE -ne 0) { throw 'npm install failed.' }
        }
        Write-Host 'tauri build --no-bundle (this is the Studio console)…'
        npm run tauri build -- --no-bundle
        if ($LASTEXITCODE -ne 0) { throw 'tauri build failed.' }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $tauriExe)) {
    # productName may produce a spaced name on some hosts
    $alt = Get-ChildItem -LiteralPath $tauriReleaseDir -Filter '*.exe' -File -ErrorAction SilentlyContinue |
        Where-Object {
            $_.Name -notmatch 'uninstall|nsis|setup' -and
            $_.Length -gt 5MB
        } |
        Sort-Object Length -Descending |
        Select-Object -First 1
    if ($alt) { $tauriExe = $alt.FullName }
}

if (-not (Test-Path -LiteralPath $tauriExe)) {
    throw "Tauri Studio executable not found under $tauriReleaseDir"
}

$tauriSize = (Get-Item -LiteralPath $tauriExe).Length
if ($tauriSize -lt 5MB) {
    throw "Refusing binary that looks like legacy WPF (size=$tauriSize). Expected Tauri exe > 5MB. Path: $tauriExe"
}

Write-Host "Studio console binary: $tauriExe ($([math]::Round($tauriSize/1MB,1)) MB)" -ForegroundColor Green

# ── 2) Embedded Python runtime (LiteLLM) — NEVER via build-portable (WPF) ───
$runtimeDir = Join-Path $projectRoot 'runtime'
$runtimeSource = $null
if (Test-Path -LiteralPath (Join-Path $runtimeDir 'python.exe')) {
    $runtimeSource = $runtimeDir
    Write-Host "Using existing project runtime: $runtimeSource"
} else {
    if ($SkipRuntimeBootstrap) {
        throw 'runtime\python.exe is missing. Remove -SkipRuntimeBootstrap or run build-embedded-runtime.ps1 first.'
    }
    $runtimeSource = Join-Path $outputRoot "studio-runtime-v$version"
    Write-Host 'Building embedded Python runtime (no C#/WPF desktop)…' -ForegroundColor Yellow
    & (Join-Path $PSScriptRoot 'build-embedded-runtime.ps1') -DestinationRuntimeDir $runtimeSource
    if ($LASTEXITCODE -ne 0) { throw 'Embedded runtime build failed.' }
}

if (-not (Test-Path -LiteralPath (Join-Path $runtimeSource 'python.exe'))) {
    throw "Runtime incomplete: $runtimeSource"
}

# ── 3) Assemble Studio payload ──────────────────────────────────────────────
foreach ($name in @(
        'run_gateway.py', 'config.yaml', 'requirements.txt', 'VERSION', 'LICENSE',
        'AGENTS.md', 'README.md', 'CHANGELOG.md', 'THIRD_PARTY_NOTICES.md'
    )) {
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

# Main app MUST be Tauri Studio — overwrite any name collision
$stageExe = Join-Path $stage 'CodexChatGateway.exe'
Copy-Item -LiteralPath $tauriExe -Destination $stageExe -Force
Copy-Item -LiteralPath $runtimeSource -Destination (Join-Path $stage 'runtime') -Recurse -Force

# Also keep original cargo name for debugging
Copy-Item -LiteralPath $tauriExe -Destination (Join-Path $stage 'codex-chat-gateway-desktop.exe') -Force

$icon = Join-Path $projectRoot 'desktop\assets\gateway-logo.ico'
if (Test-Path -LiteralPath $icon) {
    Copy-Item -LiteralPath $icon -Destination (Join-Path $stage 'gateway-logo.ico') -Force
}

# Studio launchers (never point at old WPF-only flows)
$launcher = @"
@echo off
cd /d "%~dp0"
start "" "%~dp0CodexChatGateway.exe"
"@
[IO.File]::WriteAllText((Join-Path $stage 'desktop.bat'), $launcher, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $stage '桌面版.bat'), $launcher, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $stage 'Studio.bat'), $launcher, [Text.UTF8Encoding]::new($false))

[IO.File]::WriteAllText(
    (Join-Path $stage 'STUDIO'),
    "Codex Chat Gateway Studio v$version`nUI=Tauri+LobeHub`nhttps://github.com/xuyuanzhang1122/codex-chat-gateway-windows`n",
    [Text.UTF8Encoding]::new($false)
)

# Hard verify: payload exe must match Tauri size
$stageSize = (Get-Item -LiteralPath $stageExe).Length
if ($stageSize -ne $tauriSize) {
    throw "Payload CodexChatGateway.exe size mismatch (stage=$stageSize tauri=$tauriSize). Aborting."
}
if ($stageSize -lt 5MB) {
    throw "Payload looks like legacy WPF ($stageSize bytes). Aborting."
}

Write-Host "Payload main exe verified: $stageExe ($([math]::Round($stageSize/1MB,1)) MB) = Tauri Studio" -ForegroundColor Green

# ── 4) Compile Inno Setup (Studio script only) ──────────────────────────────
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
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
}

Write-Host "Compiling Inno Studio installer…"
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

$hash = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash
[IO.File]::WriteAllText($hashPath, "$hash  $([IO.Path]::GetFileName($installerPath))`n", [Text.UTF8Encoding]::new($false))

Write-Host ''
Write-Host '========================================' -ForegroundColor Green
Write-Host ' STUDIO build complete (Tauri + LobeHub)' -ForegroundColor Green
Write-Host '========================================' -ForegroundColor Green
Write-Host "Payload:   $stage"
Write-Host "Main EXE:  $stageExe  ($([math]::Round($stageSize/1MB,1)) MB)"
Write-Host "Installer: $installerPath"
Write-Host "SHA-256:   $hash"
Write-Host ''
Write-Host 'NOTE: Do NOT use dist-installer\portable* or CodexChatGateway-Setup-v1.2.0.exe' -ForegroundColor Yellow
Write-Host '      Those are the legacy C#/WPF packages.' -ForegroundColor Yellow
Write-Host 'Run the Studio Setup above, or launch:' -ForegroundColor Yellow
Write-Host "  $stageExe" -ForegroundColor Yellow
