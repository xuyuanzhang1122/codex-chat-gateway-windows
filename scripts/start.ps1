$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'model-store.ps1')
try { $profile = Set-DefaultModelEnvironment -ProjectRoot $projectRoot }
catch { & (Join-Path $PSScriptRoot 'model-manager.ps1'); $profile = Set-DefaultModelEnvironment -ProjectRoot $projectRoot }

$hostAddress = if ($env:GATEWAY_HOST) { $env:GATEWAY_HOST } else { '127.0.0.1' }
$port = if ($env:GATEWAY_PORT) { $env:GATEWAY_PORT } else { '4000' }
if ($hostAddress -ne '127.0.0.1' -and $hostAddress -ne 'localhost') {
    throw 'GATEWAY_HOST must be 127.0.0.1 or localhost. The unauthenticated gateway must remain local.'
}

$portablePython = Join-Path $projectRoot 'runtime\python.exe'
$venvPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
if (Test-Path -LiteralPath $portablePython) {
    $python = $portablePython
} elseif (Test-Path -LiteralPath $venvPython) {
    $python = $venvPython
} else {
    throw 'Python runtime is missing. Use the portable distribution or run the development installer.'
}

Write-Host "Local gateway: http://${hostAddress}:$port/v1"
Write-Host "Codex model alias: codex-chat; upstream: $env:UPSTREAM_MODEL"
Write-Host 'Press Ctrl+C to stop.'
& $python (Join-Path $projectRoot 'run_gateway.py') --config (Join-Path $projectRoot 'config.yaml') --host $hostAddress --port $port
exit $LASTEXITCODE
