$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'model-store.ps1')

try {
    $profile = Set-DefaultModelEnvironment -ProjectRoot $projectRoot
} catch {
    Write-Host $_.Exception.Message
    & (Join-Path $PSScriptRoot 'model-manager.ps1')
    $profile = Set-DefaultModelEnvironment -ProjectRoot $projectRoot
}

$hostAddress = '127.0.0.1'
$port = '4000'
$base = "http://${hostAddress}:$port"
try {
    Invoke-RestMethod -Uri "$base/health/liveliness" -TimeoutSec 2 | Out-Null
    Write-Host "Gateway is already running: $base"
    exit 0
} catch { }

$portablePython = Join-Path $projectRoot 'runtime\pythonw.exe'
$venvPython = Join-Path $projectRoot '.venv\Scripts\pythonw.exe'
if (Test-Path -LiteralPath $portablePython) { $python = $portablePython }
elseif (Test-Path -LiteralPath $venvPython) { $python = $venvPython }
else { Write-Host 'Python runtime is missing. Use the portable distribution or run the development installer.'; exit 1 }

$runner = Join-Path $projectRoot 'run_gateway.py'
$config = Join-Path $projectRoot 'config.yaml'
$logDirectory = Join-Path $projectRoot 'logs'
$stateDirectory = Join-Path $projectRoot '.gateway'
$statePath = Join-Path $stateDirectory 'state.json'
New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $stateDirectory -Force | Out-Null
$stdout = Join-Path $logDirectory 'gateway.stdout.log'
$stderr = Join-Path $logDirectory 'gateway.stderr.log'

$arguments = @("`"$runner`"", '--config', "`"$config`"", '--host', $hostAddress, '--port', $port)
$process = Start-Process -FilePath $python -ArgumentList $arguments -WorkingDirectory $projectRoot -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
$state = [pscustomobject]@{ pid = $process.Id; executable = $python; runner = $runner; endpoint = $base; model = $profile.litellm_model; started_at = (Get-Date).ToString('o') }
[IO.File]::WriteAllText($statePath, ($state | ConvertTo-Json), [Text.UTF8Encoding]::new($false))

$ready = $false
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    Start-Sleep -Milliseconds 500
    if ($process.HasExited) { break }
    try { Invoke-RestMethod -Uri "$base/health/liveliness" -TimeoutSec 2 | Out-Null; $ready = $true; break } catch { }
}
if (-not $ready) {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
    Write-Host 'Gateway failed to start. See logs\gateway.stderr.log.'
    exit 1
}
Write-Host "Gateway started in the background: $base"
Write-Host "Default model: $($profile.name) ($($profile.litellm_model))"
Write-Host 'Closing this window will not stop the gateway.'
