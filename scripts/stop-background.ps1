$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$statePath = Join-Path $projectRoot '.gateway\state.json'
if (-not (Test-Path -LiteralPath $statePath)) { Write-Host 'Gateway is not running (no state file).'; exit 0 }
$state = Get-Content -LiteralPath $statePath -Raw -Encoding UTF8 | ConvertFrom-Json
$processInfo = Get-CimInstance Win32_Process -Filter "ProcessId = $($state.pid)" -ErrorAction SilentlyContinue
if (-not $processInfo) { Remove-Item -LiteralPath $statePath -Force; Write-Host 'Gateway was already stopped.'; exit 0 }
$expectedExe = [IO.Path]::GetFullPath([string]$state.executable)
$actualExe = if ($processInfo.ExecutablePath) { [IO.Path]::GetFullPath([string]$processInfo.ExecutablePath) } else { '' }
$runnerName = [IO.Path]::GetFileName([string]$state.runner)
if ($actualExe -ne $expectedExe -or [string]$processInfo.CommandLine -notlike "*$runnerName*") {
    Write-Host 'Refusing to stop the recorded PID because it is not this gateway process.'
    exit 1
}
Stop-Process -Id ([int]$state.pid) -Force
Remove-Item -LiteralPath $statePath -Force
Write-Host 'Gateway stopped.'
