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
$portablePython = Join-Path $projectRoot 'runtime\python.exe'
$venvPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
if (Test-Path -LiteralPath $portablePython) { $python = $portablePython }
elseif (Test-Path -LiteralPath $venvPython) { $python = $venvPython }
else { Write-Host 'Python runtime is missing.'; exit 1 }

$userProfile = [Environment]::GetFolderPath('UserProfile')
$configHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $userProfile '.codex' }
$config = Join-Path $configHome 'config.toml'
$state = Join-Path $configHome 'codex-chat-gateway-restore.json'
& $python (Join-Path $PSScriptRoot 'restore_codex.py') --config $config --state $state
exit $LASTEXITCODE
