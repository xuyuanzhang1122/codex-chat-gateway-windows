$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
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
