$ErrorActionPreference = 'Stop'
if (-not $PSScriptRoot) {
    if ($MyInvocation.MyCommand.Path) {
        $PSScriptRoot = Split-Path -Parent -LiteralPath $MyInvocation.MyCommand.Path
    } elseif ($env:CODEX_CHAT_GATEWAY_ROOT) {
        $PSScriptRoot = Join-Path $env:CODEX_CHAT_GATEWAY_ROOT 'scripts'
    } else {
        throw 'PSScriptRoot is empty; cannot locate scripts directory.'
    }
}
$projectRoot = if ($env:CODEX_CHAT_GATEWAY_ROOT) { $env:CODEX_CHAT_GATEWAY_ROOT } else { Split-Path -Parent $PSScriptRoot }
. (Join-Path $PSScriptRoot 'model-store.ps1')
$profile = Set-DefaultModelEnvironment -ProjectRoot $projectRoot
$portablePython = Join-Path $projectRoot 'runtime\python.exe'
$venvPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
if (Test-Path -LiteralPath $portablePython) { $python = $portablePython }
elseif (Test-Path -LiteralPath $venvPython) { $python = $venvPython }
else { throw 'Python runtime is missing. Use the portable release or run the development installer.' }

& $python (Join-Path $PSScriptRoot 'claude_desktop_config.py') apply --base-url 'http://127.0.0.1:4000' --model-label ([string]$profile.name)
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host 'The upstream API key was not written to Claude Desktop files.'
