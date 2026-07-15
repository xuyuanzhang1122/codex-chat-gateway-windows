$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$portablePython = Join-Path $projectRoot 'runtime\python.exe'
$venvPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
if (Test-Path -LiteralPath $portablePython) {
    $python = $portablePython
} elseif (Test-Path -LiteralPath $venvPython) {
    $python = $venvPython
} else {
    throw 'Python runtime is missing. Use the portable distribution or run the development installer.'
}

$userProfile = [Environment]::GetFolderPath('UserProfile')
$configHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $userProfile '.codex' }
$config = Join-Path $configHome 'config.toml'
$state = Join-Path $configHome 'codex-chat-gateway-restore.json'
& $python (Join-Path $PSScriptRoot 'configure_codex.py') --config $config --state $state --port 4000
exit $LASTEXITCODE
