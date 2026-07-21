param([switch]$NonInteractive)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$modelsPath = Join-Path $projectRoot '.gateway\models.json'
if (-not (Test-Path -LiteralPath $modelsPath)) {
    Write-Host 'Configure a default model in Studio before starting the gateway.'
    exit 2
}
$store = Get-Content -LiteralPath $modelsPath -Raw -Encoding UTF8 | ConvertFrom-Json
$profile = @($store.profiles | Where-Object { $_.id -eq $store.default_id } | Select-Object -First 1)
if ($profile.Count -eq 0) { $profile = @($store.profiles | Select-Object -First 1) }
if ($profile.Count -eq 0) { Write-Host 'No upstream model is configured.'; exit 2 }
$profile = $profile[0]

$hostAddress = '127.0.0.1'
$port = '4000'
$base = "http://${hostAddress}:$port"
try {
    Invoke-RestMethod -Uri "$base/health/liveliness" -TimeoutSec 2 | Out-Null
    Write-Host "Gateway is already running: $base"
    exit 0
} catch { }

$nativeGateway = Join-Path $projectRoot 'ccg-native-gateway.exe'
if (-not (Test-Path -LiteralPath $nativeGateway)) {
    $nativeGateway = Join-Path $projectRoot 'native-gateway\target\release\ccg-native-gateway.exe'
}
if (-not (Test-Path -LiteralPath $nativeGateway)) {
    Write-Host 'Native gateway is missing. Reinstall Studio or build native-gateway first.'
    exit 1
}

$logDirectory = Join-Path $projectRoot 'logs'
$stateDirectory = Join-Path $projectRoot '.gateway'
New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $stateDirectory -Force | Out-Null
$stdout = Join-Path $logDirectory 'gateway.stdout.log'
$stderr = Join-Path $logDirectory 'gateway.stderr.log'

$env:CCG_ROOT = $projectRoot
$env:CCG_PORT = $port
$process = Start-Process -FilePath $nativeGateway -WorkingDirectory $projectRoot -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru

$ready = $false
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    Start-Sleep -Milliseconds 500
    if ($process.HasExited) { break }
    try { Invoke-RestMethod -Uri "$base/health/liveliness" -TimeoutSec 2 | Out-Null; $ready = $true; break } catch { }
}
if (-not $ready) {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    Write-Host 'Gateway failed to start. See logs\gateway.stderr.log.'
    exit 1
}
Write-Host "Gateway started in the background: $base"
Write-Host "Default model: $($profile.name) ($($profile.model_id))"
Write-Host 'Closing this window will not stop the gateway.'
