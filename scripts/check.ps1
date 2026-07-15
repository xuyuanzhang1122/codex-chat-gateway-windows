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
    if ('codex-chat' -notin $names) { throw 'The model list does not contain codex-chat.' }
    Write-Host 'Model route is available: codex-chat'
    Write-Host 'Basic checks passed. Test an actual Responses request after restarting Codex.'
} catch {
    Write-Host 'Gateway check failed: the local gateway is not reachable or not ready.'
    Write-Host 'Run start-gateway.bat, then retry this check.'
    exit 1
}
