param(
    [string]$OutputDirectory = '',
    [string]$InnoCompiler = '',
    [switch]$SkipTauriBuild,
    [switch]$SkipRuntimeBootstrap
)

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

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
$stage = Join-Path $outputRoot "studio-payload-v$version"
if (Test-Path -LiteralPath $stage) {
    Remove-Item -LiteralPath $stage -Recurse -Force
}
New-Item -ItemType Directory -Path $stage -Force | Out-Null

# 1) Build Tauri release binary
$tauriExeCandidates = @(
    (Join-Path $tauriDir 'src-tauri\target\release\codex-chat-gateway-desktop.exe'),
    (Join-Path $tauriDir 'src-tauri\target\release\Codex Chat Gateway.exe')
)

if (-not $SkipTauriBuild) {
    Push-Location $tauriDir
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'node_modules'))) {
            npm install
            if ($LASTEXITCODE -ne 0) { throw 'npm install failed.' }
        }
        npm run tauri build -- --no-bundle
        if ($LASTEXITCODE -ne 0) { throw 'tauri build failed.' }
    } finally {
        Pop-Location
    }
}

$tauriExe = $tauriExeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $tauriExe) {
    # Fallback: any release exe under target/release
    $tauriExe = Get-ChildItem -LiteralPath (Join-Path $tauriDir 'src-tauri\target\release') -Filter '*.exe' -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -notmatch 'uninstall|nsis' } |
        Sort-Object Length -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $tauriExe -or -not (Test-Path -LiteralPath $tauriExe)) {
    throw 'Tauri release executable was not found. Run without -SkipTauriBuild.'
}

# 2) Ensure portable Python runtime exists (LiteLLM process)
$runtimeDir = Join-Path $projectRoot 'runtime'
if (-not (Test-Path -LiteralPath (Join-Path $runtimeDir 'python.exe'))) {
    if ($SkipRuntimeBootstrap) {
        throw 'runtime\python.exe is missing. Build portable runtime first or omit -SkipRuntimeBootstrap.'
    }
    # Reuse portable build to materialize runtime into project (heavy but correct)
    $portableOut = Join-Path $outputRoot 'portable-bootstrap'
    & (Join-Path $PSScriptRoot 'build-portable.ps1') -OutputDirectory $portableOut
    if ($LASTEXITCODE -ne 0) { throw 'Portable bootstrap failed (needed for Python runtime).' }
    $portableRoot = Join-Path $portableOut "codex-chat-gateway-portable-v$version"
    if (-not (Test-Path -LiteralPath (Join-Path $portableRoot 'runtime\python.exe'))) {
        throw "Portable runtime missing after bootstrap: $portableRoot"
    }
    # Prefer copying runtime into stage from portable; also leave project runtime if user wants
    $runtimeSource = Join-Path $portableRoot 'runtime'
} else {
    $runtimeSource = $runtimeDir
}

# 3) Assemble Studio payload (gateway files + Tauri console)
$copyRoots = @(
    'scripts',
    'patches',
    'docs'
)
foreach ($name in @('run_gateway.py', 'config.yaml', 'requirements.txt', 'VERSION', 'LICENSE', 'AGENTS.md', 'README.md', 'CHANGELOG.md', 'THIRD_PARTY_NOTICES.md')) {
    $src = Join-Path $projectRoot $name
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $stage $name) -Force
    }
}
foreach ($dir in $copyRoots) {
    $src = Join-Path $projectRoot $dir
    if (Test-Path -LiteralPath $src) {
        Copy-Item -LiteralPath $src -Destination (Join-Path $stage $dir) -Recurse -Force
    }
}

# Chinese/English launchers that point at Studio exe
Copy-Item -LiteralPath $tauriExe -Destination (Join-Path $stage 'CodexChatGateway.exe') -Force
Copy-Item -LiteralPath $runtimeSource -Destination (Join-Path $stage 'runtime') -Recurse -Force

# Desktop assets for shortcuts/icons
$icon = Join-Path $projectRoot 'desktop\assets\gateway-logo.ico'
if (Test-Path -LiteralPath $icon) {
    Copy-Item -LiteralPath $icon -Destination (Join-Path $stage 'gateway-logo.ico') -Force
}

# Write a tiny marker so app can detect Studio install
[IO.File]::WriteAllText(
    (Join-Path $stage 'STUDIO'),
    "Codex Chat Gateway Studio v$version`nhttps://github.com/xuyuanzhang1122/codex-chat-gateway-windows`n",
    [Text.UTF8Encoding]::new($false)
)

# 4) Compile Inno Setup
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
$installerPath = Join-Path $outputRoot "CodexChatGateway-Studio-Setup-v$version.exe"
$hashPath = "$installerPath.sha256"
foreach ($path in @($installerPath, $hashPath)) {
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
}

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

Write-Host ""
Write-Host "Studio payload: $stage"
Write-Host "Installer:      $installerPath"
Write-Host "SHA-256:        $hash"
Write-Host ""
Write-Host "Next (release): upload the Setup exe + sha256 to GitHub Releases."
Write-Host "Later: enable tauri-plugin-updater with a signing key and latest.json."
