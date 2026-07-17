param(
    [string]$OutputDirectory = '',
    [string]$PayloadDirectory = '',
    [string]$InnoCompiler = ''
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
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

if (-not $PayloadDirectory) {
    $portableOutput = Join-Path $outputRoot 'portable'
    & (Join-Path $PSScriptRoot 'build-portable.ps1') -OutputDirectory $portableOutput
    if ($LASTEXITCODE -ne 0) { throw 'Portable payload build failed.' }
    $PayloadDirectory = Join-Path $portableOutput "codex-chat-gateway-portable-v$version"
}
$payload = [IO.Path]::GetFullPath($PayloadDirectory)
if (-not (Test-Path -LiteralPath (Join-Path $payload 'CodexChatGateway.exe'))) {
    throw "Portable payload is incomplete: $payload"
}
if (-not (Test-Path -LiteralPath (Join-Path $payload 'runtime\python.exe'))) {
    throw "Portable Python runtime is missing: $payload"
}

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
    throw 'Inno Setup 6 or 7 was not found. Install JRSoftware.InnoSetup.7 with winget.'
}

$installerScript = Join-Path $projectRoot 'installer\CodexChatGateway.iss'
$installerPath = Join-Path $outputRoot "CodexChatGateway-Setup-v$version.exe"
$hashPath = "$installerPath.sha256"
foreach ($path in @($installerPath, $hashPath)) {
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
}

$arguments = @(
    "/DAppVersion=$version",
    "/DPayloadDir=$payload",
    "/DOutputDir=$outputRoot",
    $installerScript
)
& $InnoCompiler $arguments
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $installerPath)) {
    throw 'Installer compilation failed.'
}

$hash = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash
[IO.File]::WriteAllText($hashPath, "$hash  $([IO.Path]::GetFileName($installerPath))`n", [Text.UTF8Encoding]::new($false))
$signature = Get-AuthenticodeSignature -LiteralPath $installerPath

Write-Host "Installer: $installerPath"
Write-Host "SHA-256: $hash"
Write-Host "Signature: $($signature.Status)"
