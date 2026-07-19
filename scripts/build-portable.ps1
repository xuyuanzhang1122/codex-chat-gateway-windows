param(
    [string]$OutputDirectory = '',
    [string]$BuildPython = ''
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $projectRoot 'dist' }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$projectPath = [IO.Path]::GetFullPath($projectRoot)
if (-not $outputRoot.StartsWith($projectPath, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'OutputDirectory must be inside the project directory.'
}

$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Invalid VERSION: $version" }
$folderName = "codex-chat-gateway-portable-v$version"
$stage = Join-Path $outputRoot $folderName
$archive = Join-Path $outputRoot "$folderName-windows-x64.7z"
$shaFile = "$archive.sha256"

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
if (Test-Path -LiteralPath $stage) {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    if (-not $resolvedStage.StartsWith($outputRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to clean a stage directory outside the output directory.'
    }
    Remove-Item -LiteralPath $stage -Recurse -Force
}
foreach ($path in @($archive, $shaFile)) {
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
}
New-Item -ItemType Directory -Path $stage -Force | Out-Null

& (Join-Path $PSScriptRoot 'build-desktop.ps1') -OutputPath (Join-Path $stage 'CodexChatGateway.exe')
if ($LASTEXITCODE -ne 0) { throw 'Desktop application build failed.' }

$buildCache = Join-Path $projectRoot '.portable-build'
$pythonArchive = Join-Path $buildCache 'python-3.11.9-embed-amd64.zip'
New-Item -ItemType Directory -Path $buildCache -Force | Out-Null
if (-not (Test-Path -LiteralPath $pythonArchive)) {
    Invoke-WebRequest -Uri 'https://www.python.org/ftp/python/3.11.9/python-3.11.9-embed-amd64.zip' -OutFile $pythonArchive -UseBasicParsing
}
$pythonHash = (Get-FileHash -LiteralPath $pythonArchive -Algorithm SHA256).Hash
if ($pythonHash -ne '009D6BF7E3B2DDCA3D784FA09F90FE54336D5B60F0E0F305C37F400BF83CFD3B') {
    throw "Unexpected embedded Python SHA-256: $pythonHash"
}

$runtime = Join-Path $stage 'runtime'
Expand-Archive -LiteralPath $pythonArchive -DestinationPath $runtime -Force
$pth = Join-Path $runtime 'python311._pth'
[IO.File]::WriteAllLines($pth, @('python311.zip', '.', 'Lib\site-packages', '', 'import site'), [Text.UTF8Encoding]::new($false))
$sitePackages = Join-Path $runtime 'Lib\site-packages'
New-Item -ItemType Directory -Path $sitePackages -Force | Out-Null

if (-not $BuildPython) {
    $projectPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
    if (Test-Path -LiteralPath $projectPython) { $BuildPython = $projectPython }
    else { $BuildPython = (Get-Command python -ErrorAction Stop).Source }
}
& $BuildPython -m pip --version *> $null
if ($LASTEXITCODE -ne 0) { throw "Build Python does not provide pip: $BuildPython" }
& $BuildPython -m pip install --disable-pip-version-check --quiet --target $sitePackages -r (Join-Path $projectRoot 'requirements.txt')
if ($LASTEXITCODE -ne 0) { throw 'Dependency installation failed.' }

$rootFiles = @(
    '.gitignore', 'AGENTS.md', 'README.md', 'CHANGELOG.md', 'LICENSE',
    'THIRD_PARTY_NOTICES.md', 'VERSION',
    'config.yaml', 'run_gateway.py', 'gateway_runtime.py'
)
foreach ($name in $rootFiles) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination (Join-Path $stage $name) -Force
}

# Launchers live in bin/ for the source tree; portable package keeps them at the package root
# with paths rewritten to sit next to scripts/ (flat layout users expect).
$binDir = Join-Path $projectRoot 'bin'
$excludeLaunchers = @('desktop-tauri.bat', 'install.bat')
Get-ChildItem -LiteralPath $binDir -Filter '*.bat' -File | ForEach-Object {
    $name = $_.Name
    if ($excludeLaunchers -contains $name) { return }
    if ($name -match 'Studio|Tauri|tauri|build') { return }
    $text = [IO.File]::ReadAllText($_.FullName)
    # bin launchers point at ..\scripts; portable stage is flat next to scripts/
    $text = $text.Replace('%~dp0..\scripts\', '%~dp0scripts\')
    $text = $text.Replace('%~dp0../scripts/', '%~dp0scripts/')
    $text = $text.Replace('%~dp0..\CodexChatGateway.exe', '%~dp0CodexChatGateway.exe')
    $text = $text.Replace('%~dp0../CodexChatGateway.exe', '%~dp0CodexChatGateway.exe')
    $dest = Join-Path $stage $name
    [IO.File]::WriteAllText($dest, $text)
}

$releaseScripts = @(
    'check.ps1', 'claude_desktop_config.py', 'configure-claude-desktop.ps1',
    'configure_codex.py', 'configure-codex.ps1',
    'disable-autostart.ps1', 'enable-autostart.ps1', 'model-manager.ps1',
    'model-store.ps1', 'restore-claude-desktop.ps1', 'restore_codex.py', 'restore-codex.ps1',
    'start-background.ps1', 'status.ps1', 'stop-background.ps1'
)
$stageScripts = Join-Path $stage 'scripts'
New-Item -ItemType Directory -Path $stageScripts -Force | Out-Null
foreach ($name in $releaseScripts) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $stageScripts $name) -Force
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'patches') -Destination (Join-Path $stage 'patches') -Recurse -Force

$portablePython = Join-Path $runtime 'python.exe'
& $portablePython (Join-Path $projectRoot 'tests\test_tool_output_adjacency.py')
if ($LASTEXITCODE -ne 0) { throw 'Tool adjacency regression test failed.' }
& $portablePython (Join-Path $projectRoot 'tests\test_codex_restore.py')
if ($LASTEXITCODE -ne 0) { throw 'Codex restore regression test failed.' }
& $portablePython (Join-Path $projectRoot 'tests\test_claude_desktop_config.py')
if ($LASTEXITCODE -ne 0) { throw 'Claude Desktop config regression test failed.' }
& $portablePython (Join-Path $projectRoot 'tests\test_anthropic_gateway.py')
if ($LASTEXITCODE -ne 0) { throw 'Anthropic gateway regression test failed.' }
& $portablePython (Join-Path $projectRoot 'tests\test_gateway_routing.py')
if ($LASTEXITCODE -ne 0) { throw 'Multi-account routing regression test failed.' }
& powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File (Join-Path $projectRoot 'tests\test_model_store_v2.ps1')
if ($LASTEXITCODE -ne 0) { throw 'Model store v3 migration regression test failed.' }

foreach ($file in Get-ChildItem -LiteralPath $stageScripts -Filter '*.ps1') {
    [void][scriptblock]::Create((Get-Content -LiteralPath $file.FullName -Raw))
    $bytes = [IO.File]::ReadAllBytes($file.FullName)
    if (@($bytes | Where-Object { $_ -gt 127 }).Count -gt 0) {
        throw "PowerShell 5.1 compatibility requires ASCII-only scripts: $($file.Name)"
    }
}

$sevenZip = 'C:\Program Files\7-Zip\7z.exe'
if (-not (Test-Path -LiteralPath $sevenZip)) {
    $sevenZip = (Get-Command 7z.exe -ErrorAction Stop).Source
}
Push-Location $outputRoot
try {
    & $sevenZip a -t7z -mx=7 -mmt=on $archive $folderName
    if ($LASTEXITCODE -ne 0) { throw '7-Zip packaging failed.' }
} finally {
    Pop-Location
}

$archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
[IO.File]::WriteAllText($shaFile, "$archiveHash  $([IO.Path]::GetFileName($archive))`n", [Text.UTF8Encoding]::new($false))
Write-Host "Archive: $archive"
Write-Host "SHA-256: $archiveHash"
