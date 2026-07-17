param(
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputPath) { $OutputPath = Join-Path $projectRoot 'CodexChatGateway.exe' }
$source = Join-Path $projectRoot 'desktop\GatewayDesktop.cs'
$icon = Join-Path $projectRoot 'desktop\assets\gateway-logo.ico'
$manifest = Join-Path $projectRoot 'desktop\app.manifest'
$output = [IO.Path]::GetFullPath($OutputPath)
$outputDirectory = Split-Path -Parent $output
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

$framework = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319'
$compiler = Join-Path $framework 'csc.exe'
if (-not (Test-Path -LiteralPath $compiler)) {
    $framework = Join-Path $env:WINDIR 'Microsoft.NET\Framework\v4.0.30319'
    $compiler = Join-Path $framework 'csc.exe'
}
if (-not (Test-Path -LiteralPath $compiler)) { throw 'The .NET Framework C# compiler was not found.' }
if (-not (Test-Path -LiteralPath $icon)) { throw 'Desktop icon is missing.' }
if (-not (Test-Path -LiteralPath $manifest)) { throw 'Desktop manifest is missing.' }

$windowsBase = Get-ChildItem -LiteralPath (Join-Path $env:WINDIR 'Microsoft.NET\assembly\GAC_MSIL\WindowsBase') -Filter 'WindowsBase.dll' -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq 'WindowsBase.dll' } | Select-Object -First 1 -ExpandProperty FullName
if (-not $windowsBase) { throw 'WindowsBase.dll (WPF) was not found in the GAC.' }

$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Invalid VERSION: $version" }
$versionInfo = Join-Path $env:TEMP ("codex-gateway-versioninfo-" + [guid]::NewGuid().ToString('N') + '.cs')
$versionSource = @"
using System.Reflection;
using System.Runtime.InteropServices;
[assembly: AssemblyTitle("Codex Chat Gateway Desktop")]
[assembly: AssemblyDescription("Local model bridge console for Codex and Claude Desktop Code mode")]
[assembly: AssemblyCompany("codex-chat-gateway community")]
[assembly: AssemblyProduct("Codex Chat Gateway")]
[assembly: AssemblyCopyright("MIT License")]
[assembly: AssemblyVersion("$version.0")]
[assembly: AssemblyFileVersion("$version.0")]
[assembly: ComVisible(false)]
"@
[IO.File]::WriteAllText($versionInfo, $versionSource, [Text.UTF8Encoding]::new($false))

$references = @(
    (Join-Path $framework 'System.dll'),
    (Join-Path $framework 'System.Core.dll'),
    (Join-Path $framework 'System.Drawing.dll'),
    (Join-Path $framework 'System.Windows.Forms.dll'),
    (Join-Path $framework 'System.Web.Extensions.dll'),
    (Join-Path $framework 'System.Xaml.dll'),
    (Join-Path $framework 'WPF\PresentationCore.dll'),
    (Join-Path $framework 'WPF\PresentationFramework.dll'),
    $windowsBase
)
$arguments = @(
    '/nologo', '/target:winexe', '/optimize+', '/platform:anycpu',
    "/out:$output", "/win32icon:$icon", "/win32manifest:$manifest"
) + ($references | ForEach-Object { "/reference:$_" }) + @($source, $versionInfo)

try {
    & $compiler $arguments
    if ($LASTEXITCODE -ne 0) { throw 'Desktop application compilation failed.' }
} finally {
    Remove-Item -LiteralPath $versionInfo -Force -ErrorAction SilentlyContinue
}
Write-Host "Desktop application: $output"
