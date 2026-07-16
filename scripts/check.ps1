$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$hostAddress = '127.0.0.1'
$port = '4000'
$base = "http://${hostAddress}:$port"

try {
    $health = Invoke-RestMethod -Uri "$base/health/liveliness" -TimeoutSec 10
    Write-Host "Gateway is running: $base"
    $models = Invoke-RestMethod -Uri "$base/v1/models" -TimeoutSec 10
    $names = @($models.data | ForEach-Object id)
    $required = @('codex-chat', 'claude-sonnet-5', 'claude-opus-4-8', 'claude-haiku-4-5')
    $missing = @($required | Where-Object { $_ -notin $names })
    if ($missing.Count -gt 0) { throw "The model list is missing routes: $($missing -join ', ')" }
    Write-Host "Model routes are available: $($required -join ', ')"
    Write-Host 'Basic checks passed. Test an actual Responses request after restarting Codex.'
} catch {
    Write-Host 'Gateway check failed: the local gateway is not reachable or not ready.'
    Write-Host 'Run start-gateway.bat, then retry this check.'
    exit 1
}
