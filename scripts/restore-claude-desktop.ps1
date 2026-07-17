$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$portablePython = Join-Path $projectRoot 'runtime\python.exe'
$venvPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
if (Test-Path -LiteralPath $portablePython) { $python = $portablePython }
elseif (Test-Path -LiteralPath $venvPython) { $python = $venvPython }
else { throw 'Python runtime is missing. Use the portable release or run the development installer.' }

& $python (Join-Path $PSScriptRoot 'claude_desktop_config.py') restore
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
