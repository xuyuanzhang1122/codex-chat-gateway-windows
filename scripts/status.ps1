$ErrorActionPreference = 'SilentlyContinue'
$projectRoot = Split-Path -Parent $PSScriptRoot
$base = 'http://127.0.0.1:4000'
$statePath = Join-Path $projectRoot '.gateway\state.json'
$state = $null
if (Test-Path -LiteralPath $statePath) { $state = Get-Content -LiteralPath $statePath -Raw -Encoding UTF8 | ConvertFrom-Json }
try {
    Invoke-RestMethod -Uri "$base/health/liveliness" -TimeoutSec 3 | Out-Null
    Write-Host 'Status: RUNNING'
    Write-Host "Endpoint: $base/v1"
    if ($state) { Write-Host "PID: $($state.pid)"; Write-Host "Model: $($state.model)" }
    Write-Host "Logs: $projectRoot\logs"
    exit 0
} catch {
    Write-Host 'Status: STOPPED'
    Write-Host "Endpoint not reachable: $base"
    Write-Host 'Open Codex Chat Gateway Studio to start it.'
    exit 1
}
